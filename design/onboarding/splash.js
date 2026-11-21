// Quill first-launch splash: a deterministic prototype. Every frame depends only on `t` (seconds), so a frame
// seen in the contact sheet is the frame the player shows. Drawn with fills, strokes, clips and text only, so
// each call maps onto egui's painter when it is rebuilt in the app.
//
// Four beats and a lockup, cut hard between ink and paper:
//   A 0.00-1.75  "Describe it."     the message box grows, a request is typed and sent
//   B 1.75-4.20  "Watch it drawn."  the assistant builds a page on a board, ticking off its task list
//   C 4.20-6.30  "Make it yours."   a person restyles it: font, colour, position, theme
//   D 6.30-7.50                     the board joins a wall of designs, which folds into a point
//   E 7.50-9.60  Quill              the mark draws itself and the name closes up

const W = 1280, H = 800, DUR = 9.6;
const INK = '#0b0b0c', PAPER = '#f4f2ec', BLUE = '#3b82f6', AGENT = '#607dff';
const SANS = 'Arial, Helvetica, sans-serif', SERIF = 'Georgia, "Times New Roman", serif', HEAVY = '"Arial Black", Arial, sans-serif';

// The pictures (generated, in assets/): a cut-out stack of books and four cover designs for the page, then the
// fourteen finished designs that hang on the wall around it.
const WALL = Array.from({ length: 14 }, (_, i) => `wall${String(i + 1).padStart(2, '0')}.jpg`);
const PICTURES = ['hero.png', 'cover1.jpg', 'cover2.jpg', 'cover3.jpg', 'cover4.jpg', ...WALL].map(name => { const i = new Image(); i.src = 'assets/' + name; return i; });
/** Resolves once every picture has loaded (or failed), so a still frame is never drawn without them. */
const PICTURES_READY = Promise.all(PICTURES.map(i => i.decode().catch(() => {})));
const drawn = i => PICTURES[i].complete && PICTURES[i].naturalWidth > 0;

const cl = x => (x < 0 ? 0 : x > 1 ? 1 : x);
const lerp = (a, b, k) => a + (b - a) * k;
const out3 = x => 1 - (1 - x) ** 3;
const out5 = x => 1 - (1 - x) ** 5;
const in3 = x => x ** 3;
const io3 = x => (x < 0.5 ? 4 * x ** 3 : 1 - (-2 * x + 2) ** 3 / 2);
const back = x => 1 + 2.7 * (x - 1) ** 3 + 1.7 * (x - 1) ** 2;

// When each part of the page is drawn (beat B) and where it sits on the 800 x 500 board.
const BUILT = [2.05, 2.35, 2.65, 2.95, 3.25];
const TASKS = ['Navigation', 'Headline', 'Call to action', 'Hero picture', 'Book covers'];
const BOX = [[16, 8, 768, 40], [20, 100, 400, 122], [20, 228, 330, 120], [440, 76, 332, 264], [20, 356, 760, 136]];

function rr(c, x, y, w, h, r, fill, stroke, line) {
  c.beginPath();
  c.roundRect(x, y, Math.max(0, w), Math.max(0, h), Math.min(r, Math.max(0, w) / 2, Math.max(0, h) / 2));
  if (fill) { c.fillStyle = fill; c.fill(); }
  if (stroke) { c.strokeStyle = stroke; c.lineWidth = line || 1; c.stroke(); }
}

function text(c, s, x, y, font, color, spacing) {
  c.font = font; c.fillStyle = color; c.letterSpacing = (spacing || 0) + 'px'; c.fillText(s, x, y); c.letterSpacing = '0px';
}

// A line of big type whose words rise out of a mask one after another, and leave upward just before `tOut`.
function heading(c, t, t0, tOut, words, color, size) {
  c.font = `700 ${size}px ${SANS}`; c.letterSpacing = -size * 0.04 + 'px';
  const base = 64 + size * 0.86, leave = in3(cl((t - tOut + 0.16) / 0.16));
  let x = 64;
  words.split(' ').forEach((word, i) => {
    const k = out5(cl((t - t0 - i * 0.07) / 0.45)), w = c.measureText(word).width;
    c.save(); c.beginPath(); c.rect(x - 6, base - size * 0.9, w + 16, size * 1.14); c.clip();
    c.fillStyle = color; c.fillText(word, x, base + (1 - k) * size * 1.2 - leave * size * 1.2); c.restore();
    x += w + size * 0.24;
  });
  c.letterSpacing = '0px';
}

// The canvas's dot grid, and the three marks that say which beat is playing.
function ground(c, t, dark, beat) {
  c.fillStyle = dark ? 'rgba(255,255,255,.07)' : 'rgba(11,11,12,.1)';
  for (let y = 24; y < H; y += 32) for (let x = 24; x < W; x += 32) c.fillRect(x, y, 2, 2);
  for (let i = 0; i < 3; i++) rr(c, 64 + i * 50, 758, 42, 4, 2, i < beat ? (dark ? PAPER : INK) : dark ? '#34343a' : '#d2cec2');
  text(c, 'Esc to skip', 1140, 766, `400 15px ${SANS}`, dark ? '#7c7c85' : '#8b877c');
}

// The floating toolbar, with the tool in use lit.
function tools(c, t, dark, active, show) {
  if (show <= 0) return;
  const x = 64, y = 628 + (1 - show) * 16, ink = dark ? '#9a9aa2' : '#6b675d';
  c.save(); c.globalAlpha = show;
  rr(c, x, y, 268, 60, 16, dark ? '#1b1b1f' : '#ffffff', dark ? '#34343a' : '#dedbd2');
  for (let i = 0; i < 5; i++) {
    const cx = x + 38 + i * 48, cy = y + 30, on = i === active;
    if (on) rr(c, cx - 20, cy - 20, 40, 40, 10, BLUE);
    c.strokeStyle = c.fillStyle = on ? '#fff' : ink; c.lineWidth = 2; c.lineJoin = c.lineCap = 'round';
    c.beginPath();
    if (i === 0) { c.moveTo(cx - 6, cy - 9); c.lineTo(cx - 6, cy + 8); c.lineTo(cx - 1, cy + 4); c.lineTo(cx + 7, cy + 3); c.closePath(); c.fill(); }
    if (i === 1) { c.rect(cx - 8, cy - 8, 16, 16); c.moveTo(cx - 12, cy - 4); c.lineTo(cx + 12, cy - 4); c.moveTo(cx - 12, cy + 4); c.lineTo(cx + 12, cy + 4); c.stroke(); }
    if (i === 2) { c.moveTo(cx - 8, cy - 8); c.lineTo(cx + 8, cy - 8); c.moveTo(cx, cy - 8); c.lineTo(cx, cy + 9); c.stroke(); }
    if (i === 3) { c.roundRect(cx - 10, cy - 8, 20, 16, 3); c.moveTo(cx - 8, cy + 5); c.lineTo(cx - 2, cy - 1); c.lineTo(cx + 3, cy + 4); c.lineTo(cx + 6, cy + 1); c.lineTo(cx + 9, cy + 4); c.stroke(); }
    if (i === 4) { c.moveTo(cx - 8, cy + 8); c.lineTo(cx - 5, cy); c.lineTo(cx + 5, cy - 9); c.lineTo(cx + 9, cy - 5); c.lineTo(cx, cy + 5); c.closePath(); c.stroke(); }
  }
  c.restore();
}

function pointer(c, x, y, fill, edge, label) {
  c.save(); c.translate(x, y); c.scale(1.6, 1.6);
  c.beginPath(); c.moveTo(0, 0); c.lineTo(0, 16); c.lineTo(4.5, 12); c.lineTo(11.5, 11.5); c.closePath();
  c.fillStyle = fill; c.fill(); c.strokeStyle = edge; c.lineWidth = 1.2; c.lineJoin = 'round'; c.stroke();
  c.restore();
  if (label) {
    c.font = `700 13px ${SANS}`;
    const w = c.measureText(label).width + 16;
    rr(c, x + 16, y + 24, w, 24, 8, fill);
    text(c, label, x + 24, y + 41, `700 13px ${SANS}`, '#fff');
  }
}

function selection(c, x, y, w, h) {
  c.strokeStyle = BLUE; c.lineWidth = 1.5; c.strokeRect(x, y, w, h);
  [[x, y], [x + w, y], [x, y + h], [x + w, y + h]].forEach(([px, py]) => {
    c.fillStyle = '#fff'; c.fillRect(px - 4, py - 4, 8, 8); c.strokeRect(px - 4, py - 4, 8, 8);
  });
}

function along(keys, t) {
  if (t <= keys[0][0]) return [keys[0][1], keys[0][2]];
  for (let i = 1; i < keys.length; i++) {
    if (t < keys[i][0]) {
      const k = io3((t - keys[i - 1][0]) / (keys[i][0] - keys[i - 1][0]));
      return [lerp(keys[i - 1][1], keys[i][1], k), lerp(keys[i - 1][2], keys[i][2], k)];
    }
  }
  const last = keys[keys.length - 1];
  return [last[1], last[2]];
}

// The bookshop page, in the board's own 800 x 500 space. `s` says how far each part is drawn and how it is styled.
function board(c, s) {
  const fg = s.dark ? '#f4f2ec' : INK, mu = s.dark ? '#9a9aa2' : '#6b6b6b', bg = s.dark ? '#131316' : '#ffffff';
  if (!s.flat) { c.save(); c.shadowColor = 'rgba(0,0,0,.2)'; c.shadowBlur = 44; c.shadowOffsetY = 20; rr(c, 0, 0, 800, 500, 10, bg); c.restore(); }
  rr(c, 0, 0, 800, 500, 10, bg, s.dark ? '#2a2a2f' : '#dedbd2');
  const part = (i, draw) => {
    const k = s.k[i]; if (k <= 0) return;
    c.save();
    if (k < 1) { const [x, y, w, h] = BOX[i]; c.beginPath(); c.rect(x - 8, y - 8, (w + 16) * k, h + 16); c.clip(); }
    draw(); c.restore();
  };
  part(0, () => {
    rr(c, 28, 17, 18, 22, 3, fg);
    text(c, 'Mill Lane Books', 56, 34, `700 15px ${SANS}`, fg);
    ['Shop', 'Events', 'About'].forEach((l, i) => text(c, l, 520 + i * 62, 34, `400 14px ${SANS}`, mu));
    rr(c, 712, 14, 60, 28, 14, null, fg, 1.2); text(c, 'Visit', 727, 33, `700 13px ${SANS}`, fg);
  });
  part(1, () => {
    const font = s.heavy ? `400 44px ${HEAVY}` : `700 52px ${SERIF}`;
    text(c, 'Books worth', 28, 150, font, fg, s.heavy ? -1.5 : -1);
    text(c, 'the walk.', 28, 208, font, fg, s.heavy ? -1.5 : -1);
  });
  part(2, () => {
    text(c, 'An independent bookshop on Mill Lane,', 28, 248, `400 16px ${SANS}`, mu);
    text(c, 'open late on Thursdays.', 28, 270, `400 16px ${SANS}`, mu);
    rr(c, s.ctaX, 296, 150, 44, 22, fg);
    text(c, 'Plan a visit', s.ctaX + 32, 324, `700 15px ${SANS}`, bg);
  });
  part(3, () => {
    rr(c, 440, 84, 332, 256, 14, s.hero);
    if (drawn(0)) c.drawImage(PICTURES[0], 448, 92, 316, 237);
  });
  part(4, () => {
    text(c, 'New this week', 28, 376, `700 13px ${SANS}`, fg);
    for (let i = 0; i < 4; i++) {
      const k = out5(cl((s.covers - i * 0.07) / 0.3)); if (k <= 0) continue;
      const x = 28 + i * 184, y = 388 + (1 - k) * 16;
      c.save(); c.globalAlpha *= k; c.beginPath(); c.roundRect(x, y, 170, 96, 8); c.clip();
      c.fillStyle = ['#c8553d', '#2f5d50', '#e3b23c', '#28334a'][i]; c.fillRect(x, y, 170, 96);
      if (drawn(i + 1)) c.drawImage(PICTURES[i + 1], x, y, 170, 96);
      c.restore();
    }
  });
}

// One of the other designs on the wall, in the same 800 x 500 space. `i` counts the wall's fifteen places row by
// row; the middle one (7) is the board itself, so the fourteen pictures skip it.
function thumb(c, i) {
  const picture = 5 + (i < 7 ? i : i - 1);
  c.save(); c.beginPath(); c.roundRect(0, 0, 800, 500, 10); c.clip();
  c.fillStyle = '#1b1b1f'; c.fillRect(0, 0, 800, 500);
  if (drawn(picture)) c.drawImage(PICTURES[picture], 0, 0, 800, 500);
  c.restore();
  rr(c, 0, 0, 800, 500, 10, null, 'rgba(128,128,128,.35)', 2);
}

function sceneA(c, t) {
  c.fillStyle = INK; c.fillRect(0, 0, W, H);
  heading(c, t, 0, 1.6, 'Describe it.', PAPER, 184);
  const said = 'A landing page for a bookshop on Mill Lane';
  const grow = out5(cl((t - 0.12) / 0.5)), fold = io3(cl((t - 1.44) / 0.2));
  const w = lerp(104, 1152, grow) - 1048 * fold, x = 64 + 1048 * fold, y = 440, sx = x + w - 52;
  ['A pricing page with three plans', 'A sign-in screen for a banking app', 'A dashboard for a bike rental fleet'].forEach((idea, i) => {
    const k = out5(cl((t - 0.4 - i * 0.07) / 0.35)) * (1 - fold); if (k <= 0) return;
    c.font = `400 18px ${SANS}`;
    const iw = c.measureText(idea).width + 40, ix = 64 + [0, 300, 628][i];
    c.save(); c.globalAlpha = k; rr(c, ix, 574 + (1 - k) * 14, iw, 44, 22, null, '#34343a', 1.5); text(c, idea, ix + 20, 602 + (1 - k) * 14, `400 18px ${SANS}`, '#8a8a92'); c.restore();
  });
  if (t > 0.1) {
    rr(c, x, y, w, 104, 52, '#1b1b1f', '#3a3a40', 1.5);
    c.save(); c.beginPath(); c.rect(x + 36, y, Math.max(0, w - 130), 104); c.clip();
    const typed = said.slice(0, Math.floor(cl((t - 0.55) / 0.7) * said.length));
    c.globalAlpha = 1 - fold;
    text(c, typed, x + 44, y + 64, `400 36px ${SANS}`, '#ececee');
    if (t < 1.36 && Math.floor(t * 4) % 2 === 0) { c.font = `400 36px ${SANS}`; c.fillStyle = BLUE; c.fillRect(x + 47 + c.measureText(typed).width, y + 32, 3, 42); }
    c.restore();
    const press = 1 - 0.16 * Math.sin(Math.PI * cl((t - 1.3) / 0.16)), r = 36 * Math.min(1, grow * 1.6) * press;
    c.fillStyle = BLUE; c.beginPath(); c.arc(sx, y + 52, r, 0, 7); c.fill();
    c.strokeStyle = '#fff'; c.lineWidth = 4; c.lineCap = 'round'; c.lineJoin = 'round';
    c.beginPath(); c.moveTo(sx, y + 52 + r * 0.38); c.lineTo(sx, y + 52 - r * 0.38); c.moveTo(sx - r * 0.32, y + 52 - r * 0.06); c.lineTo(sx, y + 52 - r * 0.38); c.lineTo(sx + r * 0.32, y + 52 - r * 0.06); c.stroke();
  }
  for (let i = 0; i < 3; i++) rr(c, 64 + i * 50, 758, 42, 4, 2, i < 1 ? PAPER : '#34343a');
  text(c, 'Esc to skip', 1140, 766, `400 15px ${SANS}`, '#7c7c85');
  // Human: the send button swells over the whole window, which is the cut into the next beat.
  const iris = in3(cl((t - 1.56) / 0.19));
  if (iris > 0) { c.fillStyle = BLUE; c.beginPath(); c.arc(1164, 492, 36 + iris * 1300, 0, 7); c.fill(); }
}

// Where the board sits, and how it is styled, at time `t` (beats B to D).
function boardState(t) {
  return {
    k: BUILT.map(at => out3(cl((t - at) / 0.22))), covers: t - BUILT[4],
    heavy: t >= 4.62, dark: t >= 6.0,
    hero: t < 5.0 ? BLUE : t < 5.14 ? '#f97316' : t < 5.28 ? '#16a34a' : '#f97316',
    ctaX: lerp(28, 222, io3(cl((t - 5.5) / 0.4))),
  };
}
const boardScale = t => 1 + 0.035 * cl((t - 1.8) / 4.5);
const onBoard = (t, x, y) => { const s = boardScale(t); return [820 + (x - 400) * s, 500 + (y - 250) * s]; };

function drawBoard(c, t, lift) {
  const s = boardScale(t);
  c.save(); c.translate(820, 500 + lift); c.scale(s, s); c.translate(-400, -250);
  board(c, boardState(t));
  return () => c.restore();
}

function sceneB(c, t) {
  c.fillStyle = PAPER; c.fillRect(0, 0, W, H);
  ground(c, t, false, 2);
  const open = out5(cl((t - 1.75) / 0.3));
  if (open < 1) { c.fillStyle = BLUE; c.fillRect(0, 0, W, H); c.save(); c.beginPath(); c.arc(1164, 492, open * 1400, 0, 7); c.clip(); c.fillStyle = PAPER; c.fillRect(0, 0, W, H); ground(c, t, false, 2); c.restore(); }
  heading(c, t, 1.8, 4.2, 'Watch it drawn.', INK, 124);
  let tool = 0;
  TASKS.forEach((name, i) => {
    if (t >= BUILT[i]) tool = [1, 2, 2, 3, 1][i];
    const show = out5(cl((t - 1.9 - i * 0.05) / 0.3)); if (show <= 0) return;
    const y = 330 + i * 50 + (1 - show) * 14, done = t >= BUILT[i] + 0.22, active = !done && t >= (i ? BUILT[i - 1] + 0.22 : 1.95);
    c.save(); c.globalAlpha = show;
    if (done) {
      const pop = back(cl((t - BUILT[i] - 0.22) / 0.25));
      c.fillStyle = BLUE; c.beginPath(); c.arc(76, y - 8, 11 * pop, 0, 7); c.fill();
      c.strokeStyle = '#fff'; c.lineWidth = 2.4; c.lineCap = 'round'; c.lineJoin = 'round';
      c.beginPath(); c.moveTo(71, y - 8); c.lineTo(74.6, y - 4.2); c.lineTo(81.4, y - 12); c.stroke();
    } else {
      c.strokeStyle = '#cbc7bb'; c.lineWidth = 2; c.beginPath(); c.arc(76, y - 8, 10, 0, 7); c.stroke();
      if (active) { c.strokeStyle = BLUE; c.lineWidth = 2.6; c.lineCap = 'round'; c.beginPath(); c.arc(76, y - 8, 10, t * 9, t * 9 + 1.9); c.stroke(); }
    }
    text(c, name, 102, y, `${active || done ? 700 : 400} 22px ${SANS}`, done ? '#8b877c' : active ? INK : '#a09c91');
    c.restore();
  });
  if (t >= BUILT[4] + 0.3) tool = 0;
  tools(c, t, false, tool, out5(cl((t - 2.0) / 0.4)));
  const enter = out5(cl((t - 1.82) / 0.5));
  c.save(); c.globalAlpha = enter;
  const done = drawBoard(c, t, (1 - enter) * 110);
  BOX.forEach(([x, y, w, h], i) => { const k = out3(cl((t - BUILT[i]) / 0.22)); if (t >= BUILT[i] && t < BUILT[i] + 0.3) selection(c, x, y, w * k, h); });
  done(); c.restore();
  const keys = [[1.85, 1250, 790]];
  BOX.forEach(([x, y, w, h], i) => { keys.push([BUILT[i], ...onBoard(t, x + 6, y + 6)]); keys.push([BUILT[i] + 0.2, ...onBoard(t, x + w - 4, y + h - 4)]); });
  keys.push([3.9, 1150, 716]);
  const [px, py] = along(keys, t);
  c.save(); c.globalAlpha = cl((t - 1.9) / 0.15) * (1 - cl((t - 4.0) / 0.18)); pointer(c, px, py, AGENT, '#fff', 'Agent'); c.restore();
}

function sceneC(c, t) {
  c.fillStyle = INK; c.fillRect(0, 0, W, H);
  ground(c, t, true, 3);
  heading(c, t, 4.22, 6.3, 'Make it yours.', PAPER, 124);
  tools(c, t, true, 0, 1);
  const s = boardState(t);
  [['Font', 'Arial Black', 4.62], ['Fill', s.hero.toUpperCase(), 5.0], ['Position', 'X ' + Math.round(s.ctaX), 5.5], ['Theme', 'Dark', 6.0]].forEach(([label, value, at], i) => {
    const show = out5(cl((t - at) / 0.3)); if (show <= 0) return;
    const y = 330 + i * 50 + (1 - show) * 14;
    c.save(); c.globalAlpha = show;
    text(c, label, 64, y, `400 19px ${SANS}`, '#8a8a92');
    let x = 168;
    if (label === 'Fill') { rr(c, x, y - 19, 24, 24, 6, s.hero); x += 34; }
    text(c, value, x, y, `700 24px ${SANS}`, PAPER);
    c.restore();
  });
  const done = drawBoard(c, t, 0);
  if (t >= 4.46 && t < 4.92) selection(c, 20, 100, 400, 122);
  if (t >= 4.94 && t < 5.42) selection(c, 440, 84, 332, 256);
  if (t >= 5.46 && t < 5.98) selection(c, s.ctaX, 296, 150, 44);
  if (t >= 5.84 && t < 5.98) { c.strokeStyle = '#f0f'; c.lineWidth = 1; c.beginPath(); c.moveTo(s.ctaX + 75, 70); c.lineTo(s.ctaX + 75, 470); c.stroke(); }
  if (t >= 6.0) selection(c, 0, 0, 800, 500);
  done();
  const at = (x, y) => onBoard(t, x, y);
  let p = along([[4.2, 1180, 730], [4.5, ...at(230, 170)], [4.75, ...at(230, 170)], [4.96, ...at(610, 220)], [5.3, ...at(610, 220)], [5.48, ...at(110, 322)]], t);
  if (t >= 5.48) p = at(s.ctaX + 82, 322);
  if (t >= 5.92) p = along([[5.92, ...at(304, 322)], [6.04, ...at(700, 30)]], t);
  pointer(c, p[0], p[1], '#fff', INK);
}

function sceneD(c, t) {
  c.fillStyle = INK; c.fillRect(0, 0, W, H);
  const leave = 1 - cl((t - 6.3) / 0.3);
  c.save(); c.globalAlpha = leave; ground(c, t, true, 3); c.restore(); tools(c, t, true, 0, leave);
  const move = io3(cl((t - 6.3) / 0.6)), settle = out3(cl((t - 6.3) / 1.2)), wall = lerp(1.1, 1, settle);
  c.save(); c.translate(640, 400); c.rotate(-0.105 * move); c.scale(wall, wall);
  for (let row = 0; row < 3; row++) for (let col = 0; col < 5; col++) {
    const d = Math.abs(col - 2) + Math.abs(row - 1); if (!d) continue;
    const born = cl((t - 6.5 - d * 0.06) / 0.34), gone = 1 - in3(cl((t - 7.12 - (3 - d) * 0.03) / 0.3)); if (born <= 0 || gone <= 0) continue;
    const k = 0.26 * lerp(0.8, 1, back(born)) * gone;
    c.save(); c.globalAlpha = cl(born * 2.5); c.translate((col - 2) * 236 * gone, (row - 1) * 158 * gone); c.scale(k, k); c.translate(-400, -250);
    thumb(c, row * 5 + col); c.restore();
  }
  c.restore();
  // Human: the board itself flies from where beat C left it into the middle of the wall.
  const gone = 1 - in3(cl((t - 7.2) / 0.28)), k = lerp(boardScale(6.3), 0.26 * wall, move) * gone;
  c.save(); c.translate(lerp(820, 640, move), lerp(500, 400, move)); c.rotate(-0.105 * move); c.scale(k, k); c.translate(-400, -250);
  board(c, { ...boardState(6.3), flat: move > 0.5 }); c.restore();
  if (t > 7.3) { c.fillStyle = BLUE; c.beginPath(); c.arc(640, 400, 7 * cl((t - 7.3) / 0.1), 0, 7); c.fill(); }
}

function sceneE(c, t) {
  c.fillStyle = PAPER; c.fillRect(0, 0, W, H);
  const k = 330 / 1024;
  c.save(); c.translate(640 - 512 * k, 262 - 512 * k); c.scale(k, k);
  // The line the quill draws, with two fainter copies trailing it while it is still moving.
  const drawn = out5(cl((t - 7.52) / 0.5));
  c.lineCap = 'round';
  [[0.16, 0.16], [0.3, 0.08], [1, 0]].forEach(([alpha, lag]) => {
    const p = out5(cl((t - 7.52 - lag) / 0.5)); if (p <= 0 || (lag && drawn >= 0.995)) return;
    c.globalAlpha = alpha; c.setLineDash([760 * p, 2000]); c.strokeStyle = BLUE; c.lineWidth = 46;
    c.beginPath(); c.moveTo(182, 880); c.bezierCurveTo(380, 968, 650, 936, 856, 772); c.stroke();
  });
  c.setLineDash([]); c.globalAlpha = 1;
  // The quill swings down onto its nib.
  const swing = cl((t - 7.72) / 0.62);
  if (swing > 0) {
    c.translate(182, 880); c.rotate(-0.95 * (1 - back(swing))); c.translate(-182, -880);
    c.globalAlpha = cl(swing * 5);
    c.fillStyle = INK;
    c.fill(new Path2D('M300 772C318 520 540 250 836 188C836 470 600 716 300 772Z'));
    c.fill(new Path2D('M182 880L288 759L312 785Z'));
    c.strokeStyle = PAPER; c.lineWidth = 18;
    c.stroke(new Path2D('M676 660L500 604M808 484L634 440M338 736L716 316'));
  }
  c.restore();
  // The name: wide apart at first, then the letters close up.
  const close = out5(cl((t - 8.0) / 0.7)), gap = lerp(90, -5, close);
  c.font = `700 140px ${SANS}`;
  const widths = [...'Quill'].map(ch => c.measureText(ch).width), total = widths.reduce((a, b) => a + b, 0) + gap * 4;
  let x = 640 - total / 2;
  [...'Quill'].forEach((ch, i) => {
    c.globalAlpha = cl((t - 8.0 - i * 0.05) / 0.2); c.fillStyle = INK; c.fillText(ch, x, 572); x += widths[i] + gap;
  });
  c.globalAlpha = 1;
  const said = out5(cl((t - 8.55) / 0.45));
  c.save(); c.beginPath(); c.rect(0, 596, W, 44); c.clip(); c.textAlign = 'center';
  text(c, 'Describe it. Watch it drawn. Make it yours.', 640, 628 + (1 - said) * 44, `400 25px ${SANS}`, '#6b675d');
  c.restore();
  const ready = out5(cl((t - 8.95) / 0.4));
  if (ready > 0) {
    c.save(); c.globalAlpha = ready; c.translate(0, (1 - ready) * 14);
    rr(c, 540, 674, 200, 56, 28, INK); c.textAlign = 'center';
    text(c, 'Set up Quill', 640, 708, `700 18px ${SANS}`, PAPER);
    text(c, 'Skip setup', 640, 764, `400 15px ${SANS}`, '#8b877c');
    c.restore();
  }
}

/** Draws the frame at `t` seconds into a 1280 x 800 space on `c`. */
function drawFrame(c, t) {
  c.save(); c.textAlign = 'left'; c.textBaseline = 'alphabetic'; c.globalAlpha = 1;
  if (t < 1.75) sceneA(c, t); else if (t < 4.2) sceneB(c, t); else if (t < 6.3) sceneC(c, t); else if (t < 7.5) sceneD(c, t); else sceneE(c, Math.min(t, DUR));
  c.restore();
}
