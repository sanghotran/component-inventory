'use strict';

/* ============================================================
   Quản lý linh kiện - frontend (vanilla JS, no build step)
   Data lives in Rust (Tauri). In a plain browser (dev/test) it
   falls back to localStorage so the UI can be exercised.
   ============================================================ */

const TEXT_FIELDS = ['part_number', 'name', 'footprint', 'pitch', 'description', 'manufacturer', 'mpn', 'category', 'location'];
const FIELD_LABEL = {
  part_number: 'Part number', name: 'Name', footprint: 'Footprint', pitch: 'Pitch', quantity: 'Quantity',
  description: 'Descriptions', manufacturer: 'Manufacturer', mpn: 'Manufacturer Part Number',
  category: 'Product Category', location: 'Vị trí',
};
const KIND_LABEL = { take: 'Lấy ra', add: 'Nhập thêm', create: 'Tạo mới', edit: 'Sửa', delete: 'Xóa', import: 'Import', restore: 'Khôi phục' };
const WEEKDAYS = ['Chủ nhật', 'Thứ hai', 'Thứ ba', 'Thứ tư', 'Thứ năm', 'Thứ sáu', 'Thứ bảy'];
const LS_KEY = 'component_inventory_v1';
/* Stock levels. Thresholds: 0 | 1–10 | 11–1000 | > 1000 (negative counts as out). */
const LEVEL_LABEL = { out: 'Hết hàng', low: 'Còn ít hàng', ok: 'Còn hàng', high: 'Tồn nhiều' };
const stockLevel = (q) => (q < 1 ? 'out' : q <= 10 ? 'low' : q <= 1000 ? 'ok' : 'high');
const LS_PERSON = 'component_inventory_last_person';

/* ---------------- helpers ---------------- */
const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const fmtNum = (n) => Number(n || 0).toLocaleString('vi-VN');
const pad2 = (n) => String(n).padStart(2, '0');
const uid = () => (crypto.randomUUID ? crypto.randomUUID() : Date.now().toString(36) + Math.random().toString(36).slice(2));
const norm = (s) => String(s ?? '').trim().toLowerCase();
/* Search folding: lower-case, drop Vietnamese tone marks, đ -> d. */
const fold = (s) => String(s ?? '').normalize('NFD').replace(/[̀-ͯ]/g, '').replace(/[đĐ]/g, 'd').toLowerCase();
/* Same text with separators removed so "rc0603fr0710" finds "RC0603FR-0710KL". */
const compactOf = (s) => s.replace(/[\s\-_.\/,()#]/g, '');
/* Identity of a part: MPN when present (several parts may share a part number),
   otherwise the part number. */
const partKey = (p) => (String(p.mpn ?? '').trim() ? 'mpn:' + norm(p.mpn) : 'pn:' + norm(p.part_number));
const partLabel = (p) => p.mpn || p.part_number || p.name;
function debounce(fn, ms) { let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); }; }

function nowTs(d = new Date()) {
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}T${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
}
function parseTs(ts) {
  const m = /^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2})(?::(\d{2}))?/.exec(ts || '');
  return m ? new Date(+m[1], m[2] - 1, +m[3], +m[4], +m[5], +(m[6] || 0)) : null;
}
const fmtDate = (d) => `${pad2(d.getDate())}/${pad2(d.getMonth() + 1)}/${d.getFullYear()}`;
const fmtTime = (d) => `${pad2(d.getHours())}:${pad2(d.getMinutes())}:${pad2(d.getSeconds())}`;
const fmtWeekday = (d) => WEEKDAYS[d.getDay()];
const fmtFull = (d) => `${fmtWeekday(d)}, ${fmtDate(d)} ${fmtTime(d)}`;
const fileStamp = (d = new Date()) => `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}_${pad2(d.getHours())}${pad2(d.getMinutes())}`;
const errMsg = (e) => (e && e.message) ? e.message : String(e);

/* ---------------- backend bridge ---------------- */
const invoke = (window.__TAURI__ && window.__TAURI__.core) ? window.__TAURI__.core.invoke : null;
const IS_APP = !!invoke;

function emptyInventory() { return { version: 1, parts: [], history: [] }; }

function downloadBlob(name, text, type = 'application/json') {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([text], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 2000);
}
function pickLocalFile(accept) {
  return new Promise((resolve) => {
    const inp = document.createElement('input');
    inp.type = 'file'; inp.accept = accept;
    inp.onchange = () => resolve(inp.files[0] || null);
    inp.click();
  });
}

const backend = {
  async load() {
    if (IS_APP) return invoke('load_inventory');
    try { const raw = localStorage.getItem(LS_KEY); return raw ? JSON.parse(raw) : emptyInventory(); }
    catch { return emptyInventory(); }
  },
  async save(inv) {
    if (IS_APP) return invoke('save_inventory', { inventory: inv });
    localStorage.setItem(LS_KEY, JSON.stringify(inv));
  },
  async dataPath() {
    return IS_APP ? invoke('data_file_path') : 'localStorage của trình duyệt (chế độ xem thử, không phải bản cài đặt)';
  },
  async pickImport() {
    if (!IS_APP) throw new Error('Import Excel chỉ hoạt động trong bản ứng dụng cài trên máy.');
    return invoke('pick_import_file');
  },
  async exportReport(inv, suggestedName) {
    if (!IS_APP) throw new Error('Xuất Excel chỉ hoạt động trong bản ứng dụng cài trên máy.');
    return invoke('export_report', { inventory: inv, suggestedName });
  },
  async exportTemplate() {
    if (!IS_APP) throw new Error('Chỉ hoạt động trong bản ứng dụng cài trên máy.');
    return invoke('export_template');
  },
  async backup(inv, suggestedName) {
    if (IS_APP) return invoke('backup_inventory', { inventory: inv, suggestedName });
    downloadBlob(suggestedName, JSON.stringify(inv, null, 2));
    return suggestedName;
  },
  async restore() {
    if (IS_APP) return invoke('restore_inventory');
    const f = await pickLocalFile('.json,application/json');
    if (!f) return null;
    const data = JSON.parse(await f.text());
    if (!data || !Array.isArray(data.parts)) throw new Error('File sao lưu không đúng định dạng');
    return data;
  },
};

/* ---------------- state ---------------- */
let state = emptyInventory();
const ui = {
  tab: 'inventory',
  search: '', category: '', location: '', level: '',
  sort: { key: 'mpn', dir: 1 },
  selected: new Set(),
  hist: { search: '', kind: '', person: '' },
  lastPerson: '',
};
try { ui.lastPerson = localStorage.getItem(LS_PERSON) || ''; } catch { /* ignore */ }

function normalizePart(p) {
  const out = { id: String(p.id || uid()), quantity: Math.trunc(Number(p.quantity) || 0) };
  for (const f of TEXT_FIELDS) out[f] = String(p[f] ?? '').trim();
  return out;
}
function normalizeInventory(inv) {
  const out = emptyInventory();
  if (inv && Array.isArray(inv.parts)) out.parts = inv.parts.map(normalizePart);
  if (inv && Array.isArray(inv.history)) {
    out.history = inv.history.map((h) => ({
      id: String(h.id || uid()), ts: String(h.ts || ''), kind: String(h.kind || ''), part_id: String(h.part_id || ''),
      part_number: String(h.part_number || ''), mpn: String(h.mpn || ''), name: String(h.name || ''), delta: Math.trunc(Number(h.delta) || 0),
      after: Math.trunc(Number(h.after) || 0), person: String(h.person || ''), note: String(h.note || ''),
    }));
  }
  return out;
}

/* Serialized saves so writes never land out of order. */
let saveChain = Promise.resolve();
function commit() {
  const snapshot = JSON.parse(JSON.stringify(state));
  saveChain = saveChain
    .then(() => backend.save(snapshot))
    .then(() => { $('#status-saved').textContent = `Đã lưu lúc ${fmtTime(new Date())}`; })
    .catch((e) => { $('#status-saved').textContent = 'LỖI LƯU DỮ LIỆU'; toast('Không lưu được dữ liệu: ' + errMsg(e), 'error', 9000); });
  return saveChain;
}

function log(kind, part, delta, after, person = '', note = '', when = new Date()) {
  state.history.unshift({
    id: uid(), ts: nowTs(when), kind,
    part_id: part.id, part_number: part.part_number, mpn: part.mpn || '', name: part.name,
    delta, after, person: person.trim(), note: note.trim(),
  });
}

function rememberPerson(name) {
  ui.lastPerson = name.trim();
  try { localStorage.setItem(LS_PERSON, ui.lastPerson); } catch { /* ignore */ }
}
function knownPersons() {
  const set = new Set();
  for (const h of state.history) if (h.person) set.add(h.person);
  if (ui.lastPerson) set.add(ui.lastPerson);
  return Array.from(set).sort((a, b) => a.localeCompare(b, 'vi'));
}

/* ---------------- toasts ---------------- */
function toast(msg, type = '', ms) {
  const el = document.createElement('div');
  el.className = 'toast ' + type;
  el.textContent = msg;
  $('#toasts').appendChild(el);
  setTimeout(() => el.remove(), ms || (type === 'error' ? 7000 : 4000));
}

/* ---------------- dialog ---------------- */
const dlg = $('#dlg');
let dlgCleanup = null;
let dlgSubmit = null;

function openDialog({ title, body, foot, wide = false, onOpen, onSubmit }) {
  closeDialog();
  dlg.className = wide ? 'wide' : '';
  dlg.innerHTML = `
    <div class="dlg-head"><h2>${esc(title)}</h2><button type="button" class="small" data-close title="Đóng (Esc)">✕</button></div>
    <form class="dlg-body" id="dlg-form" novalidate>${body}<div class="err" id="dlg-err"></div></form>
    <div class="dlg-foot">${foot}</div>`;
  dlgSubmit = onSubmit || null;
  dlg.showModal();
  if (onOpen) onOpen($('#dlg-form'));
}
function closeDialog() {
  if (dlgCleanup) { try { dlgCleanup(); } catch { /* ignore */ } dlgCleanup = null; }
  dlgSubmit = null;
  if (dlg.open) dlg.close();
}
function showErr(msg) {
  const el = $('#dlg-err');
  if (!el) return;
  el.textContent = msg; el.classList.add('show');
}
function clearErr() { const el = $('#dlg-err'); if (el) { el.textContent = ''; el.classList.remove('show'); } }

dlg.addEventListener('click', (e) => {
  if (e.target.closest('[data-close]')) closeDialog();
});
dlg.addEventListener('close', () => { if (dlgCleanup) { dlgCleanup(); dlgCleanup = null; } dlgSubmit = null; });
dlg.addEventListener('submit', (e) => {
  e.preventDefault();
  clearErr();
  if (dlgSubmit) {
    try { dlgSubmit(e.target); }
    catch (err) { showErr(errMsg(err)); }
  }
});
dlg.addEventListener('input', () => clearErr());

function confirmDialog({ title = 'Xác nhận', message, okLabel = 'Đồng ý', danger = false }) {
  return new Promise((resolve) => {
    let done = false;
    const finish = (v) => { if (!done) { done = true; resolve(v); } };
    openDialog({
      title,
      body: `<div>${message}</div>`,
      foot: `<button type="button" data-close>Hủy</button>
             <button type="submit" form="dlg-form" class="${danger ? 'danger solid' : 'primary'}">${esc(okLabel)}</button>`,
      onSubmit: () => { finish(true); closeDialog(); },
    });
    dlgCleanup = () => finish(false);
    setTimeout(() => { const b = $('.dlg-foot button[type=submit]', dlg); if (b) b.focus(); }, 0);
  });
}

/* Live clock block used by take/add dialogs. Stops when the dialog closes. */
function whenBlock(label) {
  return `<div class="when">
    <div><div class="hint">${esc(label)}</div><div class="big" id="when-date">—</div></div>
    <div><div class="hint">Thứ</div><div class="big" id="when-wd">—</div></div>
    <div><div class="hint">Giờ</div><div class="big" id="when-time">—</div></div>
    <span class="live">● chạy theo đồng hồ, chốt khi bấm xác nhận</span>
  </div>`;
}
function startWhenClock() {
  const tick = () => {
    const d = new Date();
    const a = $('#when-date'), b = $('#when-wd'), c = $('#when-time');
    if (a) a.textContent = fmtDate(d);
    if (b) b.textContent = fmtWeekday(d);
    if (c) c.textContent = fmtTime(d);
  };
  tick();
  const t = setInterval(tick, 1000);
  dlgCleanup = () => clearInterval(t);
}
function partCard(p) {
  return `<div class="part-card">
    <span class="k">Manufacturer P/N</span><b>${esc(p.mpn) || '<i>(trống)</i>'}</b>
    <span class="k">Part number</span><span>${esc(p.part_number)}</span>
    <span class="k">Name</span><span>${esc(p.name)}</span>
    <span class="k">Manufacturer</span><span>${esc(p.manufacturer)}</span>
    <span class="k">Footprint / Pitch</span><span>${esc([p.footprint, p.pitch].filter(Boolean).join(' / '))}</span>
    <span class="k">Vị trí</span><span>${esc(p.location)}</span>
    <span class="k">Đang tồn</span><span><b>${fmtNum(p.quantity)}</b></span>
  </div>`;
}
function personDatalist() {
  return `<datalist id="person-list">${knownPersons().map((n) => `<option value="${esc(n)}">`).join('')}</datalist>`;
}
/* Whole numbers only: "2.5" is rejected (NaN) rather than silently misread. */
function readInt(raw) {
  const s = String(raw ?? '').trim().replace(/\s/g, '');
  if (!/^[-+]?\d+$/.test(s)) return NaN;
  return parseInt(s, 10);
}

/* ---------------- actions: take / add ---------------- */
function findPart(id) { return state.parts.find((p) => p.id === id); }

function openTake(part) {
  if (part.quantity < 1) { toast('Linh kiện này đã hết hàng, không thể lấy.', 'error'); return; }
  openDialog({
    title: 'Lấy linh kiện',
    body: partCard(part) + whenBlock('Ngày lấy') + `
      <div class="form-grid">
        <label class="field"><span class="req">Số lượng lấy</span>
          <input name="qty" type="number" inputmode="numeric" min="1" max="${part.quantity}" step="1" value="1" required></label>
        <label class="field"><span class="req">Người lấy</span>
          <input name="person" type="text" list="person-list" value="${esc(ui.lastPerson)}" autocomplete="off" placeholder="Nhập tên"></label>
        <label class="field full">Ghi chú / mục đích<textarea name="note" rows="2"></textarea></label>
      </div>${personDatalist()}
      <p class="hint">Còn lại sau khi lấy: <b id="remain">${fmtNum(part.quantity - 1)}</b></p>`,
    foot: `<button type="button" data-close>Hủy</button>
           <button type="submit" form="dlg-form" class="primary">Xác nhận lấy</button>`,
    onOpen(form) {
      startWhenClock();
      const qty = form.elements.qty;
      qty.addEventListener('input', () => {
        const n = readInt(qty.value);
        $('#remain').textContent = Number.isFinite(n) ? fmtNum(part.quantity - n) : '…';
      });
      (ui.lastPerson ? qty : form.elements.person).focus();
      qty.select();
    },
    onSubmit(form) {
      const live = findPart(part.id);
      if (!live) throw new Error('Linh kiện này đã bị xóa.');
      const n = readInt(form.elements.qty.value);
      const person = form.elements.person.value.trim();
      if (!Number.isFinite(n) || n < 1) throw new Error('Số lượng lấy phải là số nguyên từ 1 trở lên.');
      if (n > live.quantity) throw new Error(`Chỉ còn ${fmtNum(live.quantity)} cái, không thể lấy ${fmtNum(n)}.`);
      if (!person) throw new Error('Vui lòng nhập tên người lấy.');
      const when = new Date();
      live.quantity -= n;
      log('take', live, -n, live.quantity, person, form.elements.note.value, when);
      rememberPerson(person);
      commit();
      closeDialog();
      renderAll();
      toast(`${person} đã lấy ${fmtNum(n)} × ${partLabel(live)} lúc ${fmtTime(when)}. Còn lại ${fmtNum(live.quantity)}.`, 'ok');
    },
  });
}

function openAddStock(part) {
  openDialog({
    title: 'Nhập thêm linh kiện',
    body: partCard(part) + whenBlock('Ngày nhập') + `
      <div class="form-grid">
        <label class="field"><span class="req">Số lượng thêm</span>
          <input name="qty" type="number" inputmode="numeric" min="1" step="1" value="1" required></label>
        <label class="field">Người nhập
          <input name="person" type="text" list="person-list" value="${esc(ui.lastPerson)}" autocomplete="off"></label>
        <label class="field full">Ghi chú / nguồn<textarea name="note" rows="2"></textarea></label>
      </div>${personDatalist()}
      <p class="hint">Tổng tồn sau khi thêm: <b id="remain">${fmtNum(part.quantity + 1)}</b></p>`,
    foot: `<button type="button" data-close>Hủy</button>
           <button type="submit" form="dlg-form" class="primary">Xác nhận thêm</button>`,
    onOpen(form) {
      startWhenClock();
      const qty = form.elements.qty;
      qty.addEventListener('input', () => {
        const n = readInt(qty.value);
        $('#remain').textContent = Number.isFinite(n) ? fmtNum(part.quantity + n) : '…';
      });
      qty.focus(); qty.select();
    },
    onSubmit(form) {
      const live = findPart(part.id);
      if (!live) throw new Error('Linh kiện này đã bị xóa.');
      const n = readInt(form.elements.qty.value);
      if (!Number.isFinite(n) || n < 1) throw new Error('Số lượng thêm phải là số nguyên từ 1 trở lên.');
      const person = form.elements.person.value.trim();
      const when = new Date();
      live.quantity += n;
      log('add', live, n, live.quantity, person, form.elements.note.value, when);
      if (person) rememberPerson(person);
      commit();
      closeDialog();
      renderAll();
      toast(`Đã thêm ${fmtNum(n)} × ${partLabel(live)}. Tổng tồn ${fmtNum(live.quantity)}.`, 'ok');
    },
  });
}

/* ---------------- actions: create / edit / delete ---------------- */
function partForm(p) {
  const f = (key, opts = {}) => `
    <label class="field ${opts.full ? 'full' : ''}"><span class="${opts.req ? 'req' : ''}">${esc(FIELD_LABEL[key])}</span>
      ${opts.textarea
        ? `<textarea name="${key}" rows="2">${esc(p[key])}</textarea>`
        : `<input name="${key}" type="${opts.type || 'text'}" ${opts.type === 'number' ? 'min="0" step="1" inputmode="numeric"' : ''} value="${esc(p[key])}" autocomplete="off" ${opts.list ? `list="${opts.list}"` : ''}>`}
    </label>`;
  const cats = Array.from(new Set(state.parts.map((x) => x.category).filter(Boolean))).sort();
  const locs = Array.from(new Set(state.parts.map((x) => x.location).filter(Boolean))).sort();
  const fps = Array.from(new Set(state.parts.map((x) => x.footprint).filter(Boolean))).sort();
  return `<div class="form-grid">
    ${f('part_number', { req: true })}${f('name')}
    ${f('footprint', { list: 'fp-list' })}${f('pitch')}
    ${f('quantity', { type: 'number' })}${f('location', { list: 'loc-list' })}
    ${f('manufacturer')}${f('mpn')}
    ${f('category', { list: 'cat-list' })}
    ${f('description', { full: true, textarea: true })}
  </div>
  <datalist id="cat-list">${cats.map((c) => `<option value="${esc(c)}">`).join('')}</datalist>
  <datalist id="loc-list">${locs.map((c) => `<option value="${esc(c)}">`).join('')}</datalist>
  <datalist id="fp-list">${fps.map((c) => `<option value="${esc(c)}">`).join('')}</datalist>`;
}
function readPartForm(form, id) {
  const p = { id };
  for (const k of TEXT_FIELDS) p[k] = form.elements[k].value.trim();
  const q = form.elements.quantity.value.trim() === '' ? 0 : readInt(form.elements.quantity.value);
  if (!Number.isFinite(q) || q < 0) throw new Error('Quantity phải là số nguyên không âm.');
  p.quantity = q;
  if (!p.part_number) throw new Error('Part number là bắt buộc.');
  const key = partKey(p);
  const dup = state.parts.find((x) => x.id !== id && partKey(x) === key);
  if (dup) {
    throw new Error(p.mpn
      ? `Manufacturer Part Number "${p.mpn}" đã có (part number ${dup.part_number || '—'}). Hãy dùng "Thêm" trên dòng đó để cộng số lượng.`
      : `Part number "${p.part_number}" chưa có MPN đã tồn tại. Nhập Manufacturer Part Number để phân biệt hai linh kiện cùng part number.`);
  }
  return p;
}

function openNewPart() {
  const blank = normalizePart({ id: uid(), quantity: '' });
  blank.quantity = '';
  openDialog({
    title: 'Thêm linh kiện mới',
    body: partForm(blank),
    foot: `<button type="button" data-close>Hủy</button>
           <button type="submit" form="dlg-form" class="primary">Lưu</button>`,
    onOpen(form) { form.elements.part_number.focus(); },
    onSubmit(form) {
      const p = readPartForm(form, blank.id);
      state.parts.push(p);
      log('create', p, p.quantity, p.quantity, '', 'Tạo mới');
      commit();
      closeDialog();
      renderAll();
      toast(`Đã thêm ${partLabel(p)} (tồn ${fmtNum(p.quantity)}).`, 'ok');
    },
  });
}

function openEditPart(part) {
  openDialog({
    title: 'Sửa linh kiện',
    body: partForm(part),
    foot: `<button type="button" data-close>Hủy</button>
           <button type="submit" form="dlg-form" class="primary">Lưu thay đổi</button>`,
    onOpen(form) { form.elements.name.focus(); },
    onSubmit(form) {
      const live = findPart(part.id);
      if (!live) throw new Error('Linh kiện này đã bị xóa.');
      const p = readPartForm(form, live.id);
      const changed = [...TEXT_FIELDS, 'quantity'].filter((k) => String(live[k]) !== String(p[k]));
      if (!changed.length) { closeDialog(); return; }
      const delta = p.quantity - live.quantity;
      Object.assign(live, p);
      log('edit', live, delta, live.quantity, '', 'Sửa: ' + changed.map((k) => FIELD_LABEL[k]).join(', '));
      commit();
      closeDialog();
      renderAll();
      toast(`Đã cập nhật ${partLabel(live)}.`, 'ok');
    },
  });
}

async function deleteParts(ids) {
  const parts = ids.map(findPart).filter(Boolean);
  if (!parts.length) return;
  const list = parts.slice(0, 6).map((p) => `<li><b>${esc(p.part_number)}</b> · ${esc(p.mpn || '—')} · ${esc(p.name)} (tồn ${fmtNum(p.quantity)})</li>`).join('');
  const more = parts.length > 6 ? `<li>… và ${parts.length - 6} linh kiện khác</li>` : '';
  const ok = await confirmDialog({
    title: parts.length === 1 ? 'Xóa linh kiện?' : `Xóa ${parts.length} linh kiện?`,
    message: `<p>Thao tác này xóa khỏi danh sách tồn kho (lịch sử vẫn được giữ).</p><ul>${list}${more}</ul>`,
    okLabel: parts.length === 1 ? 'Xóa' : `Xóa ${parts.length} dòng`,
    danger: true,
  });
  if (!ok) return;
  const when = new Date();
  const idSet = new Set(parts.map((p) => p.id));
  for (const p of parts) log('delete', p, -p.quantity, 0, '', 'Xóa khỏi kho', when);
  state.parts = state.parts.filter((p) => !idSet.has(p.id));
  for (const id of idSet) ui.selected.delete(id);
  commit();
  renderAll();
  toast(`Đã xóa ${parts.length} linh kiện.`, 'ok');
}

/* ---------------- import ---------------- */
async function importFlow() {
  let res;
  try { res = await backend.pickImport(); }
  catch (e) { toast(errMsg(e), 'error'); return; }
  if (!res) return;

  const byKey = new Map(state.parts.map((p) => [partKey(p), p]));
  let existing = 0;
  for (const r of res.rows) if (byKey.has(partKey(r))) existing++;
  const fresh = res.rows.length - existing;

  const mappingHtml = res.mapping
    .map((m) => `<span class="k">${esc(FIELD_LABEL[m.field] || m.field)}</span><span>← cột "${esc(m.header)}"</span>`).join('');
  const unmappedHtml = res.unmapped.length
    ? `<p class="hint">Bỏ qua các cột không nhận ra: ${res.unmapped.map((u) => `<i>${esc(u)}</i>`).join(', ')}</p>` : '';
  const previewRows = res.rows.slice(0, 8).map((r) => `<tr>
      <td class="mono">${esc(r.part_number)}</td><td class="ellipsis">${esc(r.name)}</td><td>${esc(r.footprint)}</td>
      <td>${esc(r.pitch)}</td><td class="num">${r.quantity == null ? '<i>—</i>' : fmtNum(r.quantity)}</td>
      <td>${esc(r.manufacturer)}</td><td class="mono">${esc(r.mpn)}</td><td>${esc(r.category)}</td><td>${esc(r.location)}</td>
      <td>${byKey.has(partKey(r)) ? 'đã có' : '<b>mới</b>'}</td></tr>`).join('');

  openDialog({
    title: 'Xem trước import',
    wide: true,
    body: `
      <div class="summary">
        <span class="chip">📄 ${esc(res.file_name)} · sheet "${esc(res.sheet)}" · tiêu đề ở dòng ${res.header_row}</span>
        <span class="chip">${fmtNum(res.rows.length)} dòng</span>
        <span class="chip"><b>${fmtNum(fresh)}</b> mới</span>
        <span class="chip"><b>${fmtNum(existing)}</b> đã có trong kho</span>
        ${res.skipped_rows ? `<span class="chip warn">${res.skipped_rows} dòng trống bị bỏ</span>` : ''}
        ${res.bad_quantity_rows ? `<span class="chip warn">${res.bad_quantity_rows} dòng có Quantity không phải số (coi như trống)</span>` : ''}
      </div>
      <div class="mapping">${mappingHtml}</div>${unmappedHtml}
      <h3 class="mini" style="margin:12px 0 6px">Với linh kiện đã có (cùng Manufacturer Part Number, hoặc cùng Part number khi không có MPN):</h3>
      <p class="hint" style="margin:0 0 6px">Các dòng cùng Part number nhưng khác MPN được giữ thành linh kiện riêng.</p>
      <div class="radio-list">
        <label><input type="radio" name="qtyMode" value="add" checked> <span><b>Cộng dồn số lượng</b> (nhập kho thêm) — tồn mới = tồn cũ + số trong file</span></label>
        <label><input type="radio" name="qtyMode" value="replace"> <span><b>Ghi đè số lượng</b> (kiểm kê) — tồn mới = số trong file</span></label>
        <label><input type="radio" name="qtyMode" value="keep"> <span><b>Giữ nguyên số lượng</b>, chỉ cập nhật thông tin</span></label>
      </div>
      <p><label><input type="checkbox" name="fillEmpty" checked> Điền các ô đang trống (Footprint, Pitch, Hãng, MPN, Vị trí…) từ file</label></p>
      <div class="form-grid"><label class="field">Người import<input name="person" type="text" list="person-list" value="${esc(ui.lastPerson)}" autocomplete="off"></label></div>
      ${personDatalist()}
      <h3 class="mini" style="margin:12px 0 6px">Xem trước ${Math.min(8, res.rows.length)} dòng đầu</h3>
      <div class="preview-frame"><table class="grid mini"><thead><tr>
        <th>Part number</th><th>Name</th><th>Footprint</th><th>Pitch</th><th class="num">Qty</th><th>Manufacturer</th><th>MPN</th><th>Category</th><th>Vị trí</th><th></th>
      </tr></thead><tbody>${previewRows}</tbody></table></div>`,
    foot: `<span class="left">Dữ liệu hiện tại được sao lưu tự động (.bak) trước khi ghi.</span>
           <button type="button" data-close>Hủy</button>
           <button type="submit" form="dlg-form" class="primary">Import ${fmtNum(res.rows.length)} dòng</button>`,
    onSubmit(form) {
      const qtyMode = form.elements.qtyMode.value;
      const fillEmpty = form.elements.fillEmpty.checked;
      const person = form.elements.person.value.trim();
      if (person) rememberPerson(person);
      const st = applyImport(res, { qtyMode, fillEmpty, person });
      commit();
      closeDialog();
      renderAll();
      toast(`Import xong: ${fmtNum(st.created)} mới, ${fmtNum(st.updated)} cập nhật, ${fmtNum(st.qtyChanged)} thay đổi số lượng.`, 'ok', 6000);
    },
  });
}

function applyImport(res, { qtyMode, fillEmpty, person }) {
  const when = new Date();
  const byKey = new Map(state.parts.map((p) => [partKey(p), p]));
  const st = { created: 0, updated: 0, qtyChanged: 0 };
  const noteBase = `Import từ ${res.file_name}`;
  for (const r of res.rows) {
    const key = partKey(r);
    const existing = byKey.get(key);
    if (existing) {
      let touched = false;
      if (fillEmpty) {
        for (const f of TEXT_FIELDS) {
          if (f === 'part_number' || f === 'mpn') continue;
          if (!existing[f] && r[f]) { existing[f] = String(r[f]).trim(); touched = true; }
        }
      }
      if (r.quantity != null && qtyMode !== 'keep') {
        const before = existing.quantity;
        existing.quantity = qtyMode === 'add' ? before + r.quantity : r.quantity;
        if (existing.quantity !== before) {
          st.qtyChanged++;
          log('import', existing, existing.quantity - before, existing.quantity, person,
            `${noteBase} (${qtyMode === 'add' ? 'cộng dồn' : 'ghi đè'} dòng ${r.row})`, when);
        }
        touched = touched || existing.quantity !== before;
      }
      if (touched) st.updated++;
    } else {
      const p = normalizePart({ ...r, id: uid(), quantity: r.quantity ?? 0 });
      state.parts.push(p);
      byKey.set(key, p);
      log('import', p, p.quantity, p.quantity, person, `${noteBase} (dòng ${r.row}, tạo mới)`, when);
      st.created++;
    }
  }
  return st;
}

/* ---------------- export / backup / restore ---------------- */
async function exportFlow() {
  try {
    const path = await backend.exportReport(state, `TonKho_${fileStamp()}.xlsx`);
    if (path) toast('Đã xuất Excel: ' + path, 'ok', 7000);
  } catch (e) { toast('Xuất Excel thất bại: ' + errMsg(e), 'error'); }
}
async function templateFlow() {
  try {
    const path = await backend.exportTemplate();
    if (path) toast('Đã lưu file mẫu: ' + path, 'ok', 7000);
  } catch (e) { toast(errMsg(e), 'error'); }
}
async function backupFlow() {
  try {
    const path = await backend.backup(state, `SaoLuu_LinhKien_${fileStamp()}.json`);
    if (path) toast('Đã sao lưu: ' + path, 'ok', 7000);
  } catch (e) { toast('Sao lưu thất bại: ' + errMsg(e), 'error'); }
}
async function restoreFlow() {
  let data;
  try { data = await backend.restore(); }
  catch (e) { toast('Không đọc được file sao lưu: ' + errMsg(e), 'error'); return; }
  if (!data) return;
  const incoming = normalizeInventory(data);
  const ok = await confirmDialog({
    title: 'Khôi phục từ file sao lưu?',
    message: `<p>Toàn bộ dữ liệu hiện tại (<b>${fmtNum(state.parts.length)}</b> linh kiện, <b>${fmtNum(state.history.length)}</b> dòng lịch sử) sẽ bị thay bằng dữ liệu trong file (<b>${fmtNum(incoming.parts.length)}</b> linh kiện, <b>${fmtNum(incoming.history.length)}</b> dòng lịch sử).</p><p>Nên bấm "Sao lưu" dữ liệu hiện tại trước nếu chưa chắc.</p>`,
    okLabel: 'Khôi phục',
    danger: true,
  });
  if (!ok) return;
  state = incoming;
  ui.selected.clear();
  log('restore', { id: '', part_number: '', name: '' }, 0, 0, '', `Khôi phục từ file sao lưu (${fmtNum(incoming.parts.length)} linh kiện)`);
  commit();
  renderAll();
  toast('Đã khôi phục dữ liệu.', 'ok');
}

/* ---------------- rendering: inventory ---------------- */
/* ---- search: every word must match (any field, any order), accents and
   separators ignored. Folded text is cached per part and rebuilt on change. */
const searchCache = new Map();
function searchIndex(p) {
  const sig = TEXT_FIELDS.map((f) => p[f]).join('\u0002');
  let e = searchCache.get(p.id);
  if (!e || e.sig !== sig) {
    const hay = TEXT_FIELDS.map((f) => fold(p[f])).join(' \u0001 ');
    e = { sig, hay, compact: compactOf(hay) };
    searchCache.set(p.id, e);
  }
  return e;
}
function parseQuery(q) {
  return fold(q).split(/\s+/).filter(Boolean).map((t) => ({ t, c: compactOf(t) }));
}
function matchesQuery(idx, tokens) {
  return tokens.every(({ t, c }) => idx.hay.includes(t) || (c.length > 0 && idx.compact.includes(c)));
}
const collator = new Intl.Collator('vi', { numeric: true, sensitivity: 'base' });
/* Empty values sort last so parts without MPN do not crowd the top. */
function cmpText(a, b) {
  const ea = !String(a ?? '').trim(), eb = !String(b ?? '').trim();
  if (ea !== eb) return ea ? 1 : -1;
  return collator.compare(String(a ?? ''), String(b ?? ''));
}

function visibleParts() {
  const tokens = parseQuery(ui.search);
  const list = state.parts.filter((p) => {
    if (ui.category && p.category !== ui.category) return false;
    if (ui.location && p.location !== ui.location) return false;
    if (ui.level && stockLevel(p.quantity) !== ui.level) return false;
    return tokens.length === 0 || matchesQuery(searchIndex(p), tokens);
  });
  const { key, dir } = ui.sort;
  list.sort((a, b) => {
    const r = (key === 'quantity' ? a.quantity - b.quantity : cmpText(a[key], b[key])) * dir;
    if (r) return r;
    for (const k of ['mpn', 'part_number', 'name']) {
      if (k === key) continue;
      const t = cmpText(a[k], b[k]);
      if (t) return t;
    }
    return 0;
  });
  return list;
}

function renderParts() {
  const list = visibleParts();
  const body = $('#parts-body');
  body.innerHTML = list.map((p) => `
    <tr data-id="${p.id}" class="${ui.selected.has(p.id) ? 'selected' : ''} ${p.quantity < 1 ? 'out' : ''}">
      <td class="col-check"><input type="checkbox" data-select="${p.id}" ${ui.selected.has(p.id) ? 'checked' : ''}></td>
      <td class="mono">${esc(p.part_number)}</td>
      <td class="ellipsis" title="${esc(p.name)}">${esc(p.name)}</td>
      <td>${esc(p.footprint)}</td>
      <td>${esc(p.pitch)}</td>
      <td class="num qty lvl-${stockLevel(p.quantity)}" title="${LEVEL_LABEL[stockLevel(p.quantity)]}"><div class="qty-cell"><span class="lvl-label">${LEVEL_LABEL[stockLevel(p.quantity)]}</span><span class="qty-pill">${fmtNum(p.quantity)}</span></div></td>
      <td class="ellipsis" title="${esc(p.description)}">${esc(p.description)}</td>
      <td>${esc(p.manufacturer)}</td>
      <td class="mono">${esc(p.mpn)}</td>
      <td>${esc(p.category)}</td>
      <td>${esc(p.location)}</td>
      <td class="col-actions"><div class="actions">
        <button type="button" class="small btn-take" data-action="take" ${p.quantity < 1 ? 'disabled title="Đã hết hàng"' : ''}>Lấy</button>
        <button type="button" class="small btn-add" data-action="add">Thêm</button>
        <button type="button" class="small" data-action="edit">Sửa</button>
        <button type="button" class="small danger" data-action="delete">Xóa</button>
      </div></td>
    </tr>`).join('');

  const empty = $('#parts-empty');
  if (!state.parts.length) {
    empty.textContent = 'Chưa có linh kiện nào. Bấm "Thêm linh kiện" hoặc "Import Excel/CSV" để bắt đầu.';
    empty.hidden = false;
  } else if (!list.length) {
    empty.textContent = 'Không có linh kiện nào khớp bộ lọc.';
    empty.hidden = false;
  } else empty.hidden = true;

  // select-all reflects the visible rows
  const visibleIds = list.map((p) => p.id);
  const selVisible = visibleIds.filter((id) => ui.selected.has(id)).length;
  const sa = $('#select-all');
  sa.checked = visibleIds.length > 0 && selVisible === visibleIds.length;
  sa.indeterminate = selVisible > 0 && selVisible < visibleIds.length;
  sa.disabled = visibleIds.length === 0;

  const del = $('#btn-delete-selected');
  del.disabled = ui.selected.size === 0;
  del.textContent = ui.selected.size ? `Xóa đã chọn (${ui.selected.size})` : 'Xóa đã chọn';

  $$('#parts-table thead th[data-sort]').forEach((th) => {
    const mark = th.querySelector('.sort-mark');
    if (mark) mark.remove();
    if (th.dataset.sort === ui.sort.key) th.insertAdjacentHTML('beforeend', `<span class="sort-mark">${ui.sort.dir > 0 ? '▲' : '▼'}</span>`);
  });

  const total = state.parts.reduce((s, p) => s + p.quantity, 0);
  const out = state.parts.filter((p) => stockLevel(p.quantity) === 'out').length;
  const low = state.parts.filter((p) => stockLevel(p.quantity) === 'low').length;
  $('#status-counts').innerHTML =
    `${fmtNum(state.parts.length)} linh kiện · tổng tồn ${fmtNum(total)}` +
    (out ? ` · <span class="status-out">${fmtNum(out)} hết hàng</span>` : '') +
    (low ? ` · <span class="status-low">${fmtNum(low)} còn ít</span>` : '') +
    (list.length !== state.parts.length ? ` · đang hiển thị ${fmtNum(list.length)}` : '');
}

function renderFilters() {
  const fill = (sel, values, current, allLabel) => {
    const opts = Array.from(new Set(values.filter(Boolean))).sort((a, b) => a.localeCompare(b, 'vi'));
    sel.innerHTML = `<option value="">${allLabel}</option>` + opts.map((v) => `<option value="${esc(v)}">${esc(v)}</option>`).join('');
    sel.value = opts.includes(current) ? current : '';
    return sel.value;
  };
  ui.category = fill($('#filter-category'), state.parts.map((p) => p.category), ui.category, 'Tất cả danh mục');
  ui.location = fill($('#filter-location'), state.parts.map((p) => p.location), ui.location, 'Tất cả vị trí');
  ui.hist.person = fill($('#hist-person'), state.history.map((h) => h.person), ui.hist.person, 'Mọi người');
}

/* ---------------- rendering: history ---------------- */
function renderHistory() {
  const tokens = parseQuery(ui.hist.search);
  const list = state.history.filter((h) => {
    if (ui.hist.kind && h.kind !== ui.hist.kind) return false;
    if (ui.hist.person && h.person !== ui.hist.person) return false;
    if (!tokens.length) return true;
    const d = parseTs(h.ts);
    const hay = fold([h.part_number, h.mpn, h.name, h.person, h.note, KIND_LABEL[h.kind] || h.kind,
      d ? fmtDate(d) : h.ts, d ? fmtWeekday(d) : ''].join(' \u0001 '));
    return matchesQuery({ hay, compact: compactOf(hay) }, tokens);
  });
  $('#history-body').innerHTML = list.map((h) => {
    const d = parseTs(h.ts);
    const deltaCls = h.delta > 0 ? 'delta-pos' : h.delta < 0 ? 'delta-neg' : '';
    const deltaTxt = h.delta > 0 ? '+' + fmtNum(h.delta) : fmtNum(h.delta);
    return `<tr>
      <td>${d ? fmtDate(d) : esc(h.ts)}</td>
      <td>${d ? fmtWeekday(d) : ''}</td>
      <td class="mono">${d ? fmtTime(d) : ''}</td>
      <td><span class="kind kind-${esc(h.kind)}">${esc(KIND_LABEL[h.kind] || h.kind)}</span></td>
      <td class="mono">${esc(h.part_number)}</td>
      <td class="mono">${esc(h.mpn)}</td>
      <td class="ellipsis" title="${esc(h.name)}">${esc(h.name)}</td>
      <td class="num ${deltaCls}">${deltaTxt}</td>
      <td class="num">${fmtNum(h.after)}</td>
      <td>${esc(h.person)}</td>
      <td class="ellipsis" title="${esc(h.note)}">${esc(h.note)}</td>
    </tr>`;
  }).join('');
  $('#history-empty').hidden = list.length > 0;
  $('#history-empty').textContent = state.history.length ? 'Không có dòng nào khớp bộ lọc.' : 'Chưa có thao tác nào.';
}

function renderAll() {
  renderFilters();
  renderParts();
  renderHistory();
}

/* ---------------- wiring ---------------- */
function switchTab(name) {
  ui.tab = name;
  $$('.tab-btn').forEach((b) => b.classList.toggle('active', b.dataset.tab === name));
  $$('.tab').forEach((s) => s.classList.toggle('active', s.id === 'tab-' + name));
}

function wire() {
  $$('.tab-btn').forEach((b) => b.addEventListener('click', () => switchTab(b.dataset.tab)));

  const runSearch = debounce(() => { ui.search = $('#search').value; renderParts(); }, 120);
  $('#search').addEventListener('input', runSearch);
  $('#search').addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && e.target.value) { e.preventDefault(); e.target.value = ''; ui.search = ''; renderParts(); }
  });
  $('#filter-category').addEventListener('change', (e) => { ui.category = e.target.value; renderParts(); });
  $('#filter-location').addEventListener('change', (e) => { ui.location = e.target.value; renderParts(); });
  $('#filter-level').addEventListener('change', (e) => { ui.level = e.target.value; renderParts(); });

  $('#btn-new').addEventListener('click', openNewPart);
  $('#btn-import').addEventListener('click', importFlow);
  $('#btn-export').addEventListener('click', exportFlow);
  $('#btn-export-2').addEventListener('click', exportFlow);
  $('#btn-template').addEventListener('click', () => { hideMenu(); templateFlow(); });
  $('#btn-backup').addEventListener('click', () => { hideMenu(); backupFlow(); });
  $('#btn-restore').addEventListener('click', () => { hideMenu(); restoreFlow(); });
  $('#btn-delete-selected').addEventListener('click', () => deleteParts(Array.from(ui.selected)));

  const menu = $('#menu-more');
  const hideMenu = () => { menu.hidden = true; };
  $('#btn-more').addEventListener('click', (e) => { e.stopPropagation(); menu.hidden = !menu.hidden; });
  document.addEventListener('click', (e) => { if (!e.target.closest('.menu')) hideMenu(); });

  $('#select-all').addEventListener('change', (e) => {
    const ids = visibleParts().map((p) => p.id);
    if (e.target.checked) ids.forEach((id) => ui.selected.add(id));
    else ids.forEach((id) => ui.selected.delete(id));
    renderParts();
  });

  $('#parts-table').addEventListener('click', (e) => {
    const th = e.target.closest('th[data-sort]');
    if (th) {
      const key = th.dataset.sort;
      if (ui.sort.key === key) ui.sort.dir = -ui.sort.dir; else ui.sort = { key, dir: 1 };
      renderParts();
      return;
    }
    const btn = e.target.closest('button[data-action]');
    if (btn) {
      const id = btn.closest('tr').dataset.id;
      const part = findPart(id);
      if (!part) return;
      switch (btn.dataset.action) {
        case 'take': openTake(part); break;
        case 'add': openAddStock(part); break;
        case 'edit': openEditPart(part); break;
        case 'delete': deleteParts([id]); break;
      }
    }
  });
  $('#parts-table').addEventListener('change', (e) => {
    const cb = e.target.closest('input[data-select]');
    if (!cb) return;
    if (cb.checked) ui.selected.add(cb.dataset.select); else ui.selected.delete(cb.dataset.select);
    renderParts();
  });
  // double-click a row to edit
  $('#parts-body').addEventListener('dblclick', (e) => {
    if (e.target.closest('button, input')) return;
    const tr = e.target.closest('tr[data-id]');
    if (tr) { const p = findPart(tr.dataset.id); if (p) openEditPart(p); }
  });

  const runHistSearch = debounce(() => { ui.hist.search = $('#hist-search').value; renderHistory(); }, 120);
  $('#hist-search').addEventListener('input', runHistSearch);
  $('#hist-search').addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && e.target.value) { e.preventDefault(); e.target.value = ''; ui.hist.search = ''; renderHistory(); }
  });
  $('#hist-kind').addEventListener('change', (e) => { ui.hist.kind = e.target.value; renderHistory(); });
  $('#hist-person').addEventListener('change', (e) => { ui.hist.person = e.target.value; renderHistory(); });

  document.addEventListener('keydown', (e) => {
    if (dlg.open) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') { e.preventDefault(); switchTab('inventory'); $('#search').focus(); $('#search').select(); }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') { e.preventDefault(); openNewPart(); }
  });

  const clock = $('#clock');
  const tick = () => { clock.textContent = fmtFull(new Date()); };
  tick(); setInterval(tick, 1000);
}

async function init() {
  wire();
  try {
    state = normalizeInventory(await backend.load());
  } catch (e) {
    toast('Không đọc được dữ liệu đã lưu: ' + errMsg(e) + '. Ứng dụng khởi động với danh sách trống; dữ liệu cũ KHÔNG bị ghi đè cho đến khi bạn thay đổi gì đó.', 'error', 15000);
    state = emptyInventory();
  }
  try { $('#status-path').textContent = 'Dữ liệu: ' + await backend.dataPath(); } catch { /* ignore */ }
  renderAll();
}

init();
