<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { get } from "svelte/store";
  import * as store from "../lib/store";
  import { api } from "../lib/api";
  import { arabicDigits, NOUNS, plural } from "../lib/format";
  import BackButton from "./BackButton.svelte";

  type Tool =
    | "select"
    | "hand"
    | "rect"
    | "ellipse"
    | "diamond"
    | "arrow"
    | "line"
    | "draw"
    | "text"
    | "sticky"
    | "todo"
    | "idea"
    | "mind"
    | "eraser";

  interface Point {
    x: number;
    y: number;
  }

  interface TodoItem {
    t: string;
    done: boolean;
  }

  interface Shape {
    id: number;
    type: Tool | "mindedge";
    x: number;
    y: number;
    x2: number;
    y2: number;
    color: string;
    bg: string;
    sw: number;
    text?: string;
    points?: Point[];
    items?: TodoItem[];
    from?: number;
    to?: number;
  }

  interface EditorState {
    left: number;
    top: number;
    width: number;
    height: number;
    value: string;
    shapeId: number | null;
    multiline: boolean;
  }

  const STORAGE_KEY = "noterust.board";
  const TOOLS: [Tool, string, string][] = [
    ["select", "➤", "تحديد (V)"],
    ["hand", "✋", "تحريك (H)"],
    ["rect", "▢", "مستطيل (R)"],
    ["ellipse", "○", "دائرة (O)"],
    ["diamond", "◇", "معيّن (D)"],
    ["arrow", "→", "سهم (A)"],
    ["line", "─", "خط (L)"],
    ["draw", "✎", "رسم حر (P)"],
    ["text", "A", "نص (T)"],
    ["sticky", "▦", "ملاحظة (N)"],
    ["todo", "☑", "قائمة مهام (U)"],
    ["idea", "💡", "بطاقة فكرة (F)"],
    ["mind", "🧠", "خريطة ذهنية (M)"],
    ["eraser", "⌫", "ممحاة (E)"],
  ];
  const HSV = ["#ff7979", "#f6e58d", "#7bed9f", "#70a1ff", "#e056fd", "#ffffff"];
  const FILLS = ["transparent", "#2f3542", "#6c5ce7", "#00cec9", "#ff7675"];

  let host: HTMLDivElement | null = null;
  let canvas: HTMLCanvasElement | null = null;
  let editorEl: HTMLTextAreaElement | null = null;

  let tool: Tool = "select";
  let shapes: Shape[] = [];
  let selection = new Set<number>();
  let camera = { x: 0, y: 0, z: 1 };
  let grid = true;
  let background = "#121212";
  let stroke = "#ffffff";
  let fill = "transparent";
  let strokeWidth = 2.5;

  let editor: EditorState | null = null;
  let dragging: { sx: number; sy: number; moved: boolean; before: string } | null = null;
  let marquee: { x: number; y: number; x2: number; y2: number } | null = null;
  let panning: { sx: number; sy: number; ox: number; oy: number } | null = null;
  let draft: Shape | null = null;
  let spaceHeld = false;
  let seq = 1;
  let undoStack: string[] = [];
  let redoStack: string[] = [];
  let version = 0;
  let fileInput: HTMLInputElement | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  /** Disk autosave state, shown in the toolbar. */
  let saving = false;
  let savedAt: number | null = null;

  const HISTORY_LIMIT = 60;

  $: count = shapes.filter((shape) => shape.type !== "mindedge").length;
  $: selectedShape = selection.size === 1 ? (shapes.find((s) => selection.has(s.id)) ?? null) : null;
  $: canUndo = undoStack.length > 0;
  $: canRedo = redoStack.length > 0;

  onMount(() => {
    void (async () => {
      await restoreBoard();
      if (shapes.length === 0) {
        seed();
      } else {
        // Keep a durable copy on disk right away (also migrates old boards that
        // only ever lived in localStorage).
        persist();
      }
      draw();
    })();
    const resize = () => {
      if (!canvas || !host) return;
      const rect = host.getBoundingClientRect();
      const dpr = Math.min(devicePixelRatio || 1, 2);
      canvas.width = Math.max(1, Math.round(rect.width * dpr));
      canvas.height = Math.max(1, Math.round(rect.height * dpr));
      draw();
    };
    resize();
    window.addEventListener("resize", resize);
    window.addEventListener("keydown", onKeydown);
    window.addEventListener("keyup", onKeyup);
    return () => {
      window.removeEventListener("resize", resize);
      window.removeEventListener("keydown", onKeydown);
      window.removeEventListener("keyup", onKeyup);
    };
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    persist();
  });

  // ------------------------------------------------------------------ storage

  function applyBoard(raw: string) {
    const data = JSON.parse(raw) as {
      shapes?: Shape[];
      camera?: typeof camera;
      grid?: boolean;
      background?: string;
    };
    shapes = Array.isArray(data.shapes) ? data.shapes : [];
    if (data.camera) camera = { ...camera, ...data.camera };
    grid = data.grid !== false;
    background = data.background ?? background;
    seq = shapes.reduce((max, shape) => Math.max(max, shape.id || 0), 0) + 1;
  }

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) applyBoard(raw);
    } catch {
      /* ignore corrupt board */
    }
  }

  /** Board lives on disk (auto-saved) with localStorage as a fast local copy. */
  async function restoreBoard() {
    try {
      const raw = await api.loadBoard();
      if (raw) {
        applyBoard(raw);
        savedAt = Date.now();
        return;
      }
    } catch {
      /* fall back to the local copy */
    }
    load();
  }

  function persist() {
    const payload = JSON.stringify({ shapes, camera, grid, background, version });
    try {
      localStorage.setItem(STORAGE_KEY, payload);
    } catch {
      /* storage full: keep working in memory */
    }
    void saveToDisk(payload);
  }

  async function saveToDisk(payload: string) {
    saving = true;
    try {
      await api.saveBoard(payload);
      savedAt = Date.now();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    } finally {
      saving = false;
    }
  }

  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(persist, 400);
  }

  function seed() {
    shapes = [
      { id: seq++, type: "text", x: 40, y: 60, x2: 40, y2: 60, color: "#ffffff", bg: "transparent", sw: 2.5, text: "سبورتك الأولى — ارسم فكرة أو خطة" },
      { id: seq++, type: "rect", x: 40, y: 120, x2: 240, y2: 210, color: "#70a1ff", bg: "transparent", sw: 2.5, text: "فكرة" },
      { id: seq++, type: "arrow", x: 250, y: 165, x2: 360, y2: 165, color: "#f6e58d", bg: "transparent", sw: 2.5 },
      { id: seq++, type: "ellipse", x: 370, y: 120, x2: 560, y2: 210, color: "#7bed9f", bg: "transparent", sw: 2.5, text: "خطوة" },
    ];
    persist();
  }

  // ------------------------------------------------------------------ history

  function snapshot() {
    undoStack.push(JSON.stringify(shapes));
    if (undoStack.length > HISTORY_LIMIT) undoStack.shift();
    redoStack = [];
  }

  function undo() {
    const previous = undoStack.pop();
    if (!previous) return;
    redoStack.push(JSON.stringify(shapes));
    shapes = JSON.parse(previous) as Shape[];
    selection.clear();
    scheduleSave();
    draw();
  }

  function redo() {
    const next = redoStack.pop();
    if (!next) return;
    undoStack.push(JSON.stringify(shapes));
    shapes = JSON.parse(next) as Shape[];
    selection.clear();
    scheduleSave();
    draw();
  }

  // ---------------------------------------------------------------- geometry

  function toWorld(clientX: number, clientY: number): Point {
    const rect = canvas?.getBoundingClientRect();
    const left = rect?.left ?? 0;
    const top = rect?.top ?? 0;
    return { x: (clientX - left - camera.x) / camera.z, y: (clientY - top - camera.y) / camera.z };
  }

  function toScreen(x: number, y: number): Point {
    return { x: x * camera.z + camera.x, y: y * camera.z + camera.y };
  }

  function box(shape: Shape) {
    if (shape.type === "mindedge") {
      const from = shapes.find((s) => s.id === shape.from);
      const to = shapes.find((s) => s.id === shape.to);
      if (!from || !to) return { x: 0, y: 0, w: 0, h: 0 };
      const a = { x: (from.x + from.x2) / 2, y: (from.y + from.y2) / 2 };
      const b = { x: (to.x + to.x2) / 2, y: (to.y + to.y2) / 2 };
      return {
        x: Math.min(a.x, b.x),
        y: Math.min(a.y, b.y),
        w: Math.abs(a.x - b.x),
        h: Math.abs(a.y - b.y),
      };
    }
    if (shape.type === "draw" && shape.points?.length) {
      const xs = shape.points.map((p) => p.x);
      const ys = shape.points.map((p) => p.y);
      const x = Math.min(...xs);
      const y = Math.min(...ys);
      return { x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y };
    }
    const x1 = Math.min(shape.x, shape.x2);
    const y1 = Math.min(shape.y, shape.y2);
    const x2 = Math.max(shape.x, shape.x2);
    const y2 = Math.max(shape.y, shape.y2);
    return { x: x1, y: y1, w: Math.max(4, x2 - x1), h: Math.max(4, y2 - y1) };
  }

  function shapeHeight(shape: Shape): number {
    if (shape.type === "todo") return 40 + (shape.items?.length ?? 0) * 34;
    return box(shape).h;
  }

  function hitTest(point: Point): Shape | null {
    for (let index = shapes.length - 1; index >= 0; index -= 1) {
      const shape = shapes[index];
      if (shape.type === "mindedge") continue;
      const area = box(shape);
      const height = shapeHeight(shape);
      const pad = 8 / camera.z;
      if (
        point.x >= area.x - pad &&
        point.x <= area.x + area.w + pad &&
        point.y >= area.y - pad &&
        point.y <= area.y + height + pad
      ) {
        return shape;
      }
    }
    return null;
  }

  // ----------------------------------------------------------------- drawing

  function context(): CanvasRenderingContext2D | null {
    return canvas?.getContext("2d") ?? null;
  }

  function draw() {
    const ctx = context();
    if (!ctx || !canvas) return;
    const dpr = Math.min(devicePixelRatio || 1, 2);
    const width = canvas.width / dpr;
    const height = canvas.height / dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = background;
    ctx.fillRect(0, 0, width, height);

    if (grid) {
      const step = 28 * camera.z;
      if (step > 6) {
        ctx.strokeStyle = "rgba(255,255,255,0.06)";
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let x = ((camera.x % step) + step) % step; x < width; x += step) {
          ctx.moveTo(x, 0);
          ctx.lineTo(x, height);
        }
        for (let y = ((camera.y % step) + step) % step; y < height; y += step) {
          ctx.moveTo(0, y);
          ctx.lineTo(width, y);
        }
        ctx.stroke();
      }
    }

    ctx.save();
    ctx.translate(camera.x, camera.y);
    ctx.scale(camera.z, camera.z);
    ctx.lineJoin = "round";
    ctx.lineCap = "round";

    for (const shape of shapes) paint(ctx, shape);
    if (draft) paint(ctx, draft);

    if (selection.size > 0) {
      ctx.save();
      ctx.strokeStyle = "#8f8aff";
      ctx.setLineDash([6 / camera.z, 4 / camera.z]);
      ctx.lineWidth = 1.4 / camera.z;
      for (const id of selection) {
        const shape = shapes.find((item) => item.id === id);
        if (!shape) continue;
        const area = box(shape);
        ctx.strokeRect(area.x - 7, area.y - 7, area.w + 14, shapeHeight(shape) + 14);
      }
      ctx.restore();
    }

    if (marquee) {
      const x = Math.min(marquee.x, marquee.x2);
      const y = Math.min(marquee.y, marquee.y2);
      ctx.fillStyle = "rgba(105,101,219,.14)";
      ctx.fillRect(x, y, Math.abs(marquee.x2 - marquee.x), Math.abs(marquee.y2 - marquee.y));
      ctx.strokeStyle = "#8f8aff";
      ctx.lineWidth = 1.4 / camera.z;
      ctx.strokeRect(x, y, Math.abs(marquee.x2 - marquee.x), Math.abs(marquee.y2 - marquee.y));
    }
    ctx.restore();
  }

  function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    const radius = Math.min(r, w / 2, h / 2);
    ctx.beginPath();
    ctx.moveTo(x + radius, y);
    ctx.arcTo(x + w, y, x + w, y + h, radius);
    ctx.arcTo(x + w, y + h, x, y + h, radius);
    ctx.arcTo(x, y + h, x, y, radius);
    ctx.arcTo(x, y, x + w, y, radius);
    ctx.closePath();
  }

  function paintText(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, maxWidth: number, align: "center" | "start") {
    const words = String(text).split(/\s+/);
    const lines: string[] = [];
    let line = "";
    for (const word of words) {
      const candidate = line ? `${line} ${word}` : word;
      if (ctx.measureText(candidate).width > maxWidth && line) {
        lines.push(line);
        line = word;
      } else {
        line = candidate;
      }
    }
    if (line) lines.push(line);
    const lineHeight = 26;
    let cursor = y - ((lines.length - 1) * lineHeight) / 2;
    for (const item of lines) {
      if (align === "center") ctx.fillText(item, x, cursor);
      else {
        ctx.textAlign = "start";
        ctx.fillText(item, x, cursor);
        ctx.textAlign = "center";
      }
      cursor += lineHeight;
    }
  }

  function paint(ctx: CanvasRenderingContext2D, shape: Shape) {
    const area = box(shape);
    ctx.save();
    ctx.strokeStyle = shape.color || "#fff";
    ctx.lineWidth = shape.sw || 2;
    ctx.fillStyle = shape.bg && shape.bg !== "transparent" ? shape.bg : "transparent";

    if (shape.type === "mindedge") {
      const from = shapes.find((s) => s.id === shape.from);
      const to = shapes.find((s) => s.id === shape.to);
      if (from && to) {
        const a = { x: (from.x + from.x2) / 2, y: (from.y + from.y2) / 2 };
        const b = { x: (to.x + to.x2) / 2, y: (to.y + to.y2) / 2 };
        const bend = Math.max(30, Math.abs(b.x - a.x) / 2);
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.bezierCurveTo(a.x + bend, a.y, b.x - bend, b.y, b.x, b.y);
        ctx.stroke();
      }
      ctx.restore();
      return;
    }

    if (shape.type === "draw" && shape.points?.length) {
      ctx.beginPath();
      ctx.moveTo(shape.points[0].x, shape.points[0].y);
      for (const point of shape.points.slice(1)) ctx.lineTo(point.x, point.y);
      ctx.stroke();
      ctx.restore();
      return;
    }

    if (shape.type === "arrow" || shape.type === "line") {
      ctx.beginPath();
      ctx.moveTo(shape.x, shape.y);
      ctx.lineTo(shape.x2, shape.y2);
      ctx.stroke();
      if (shape.type === "arrow") {
        const angle = Math.atan2(shape.y2 - shape.y, shape.x2 - shape.x);
        const length = 14;
        ctx.beginPath();
        ctx.moveTo(shape.x2, shape.y2);
        ctx.lineTo(shape.x2 - length * Math.cos(angle - 0.4), shape.y2 - length * Math.sin(angle - 0.4));
        ctx.moveTo(shape.x2, shape.y2);
        ctx.lineTo(shape.x2 - length * Math.cos(angle + 0.4), shape.y2 - length * Math.sin(angle + 0.4));
        ctx.stroke();
      }
      ctx.restore();
      return;
    }

    if (shape.type === "rect" || shape.type === "sticky" || shape.type === "idea" || shape.type === "todo") {
      if (shape.type === "sticky") ctx.fillStyle = shape.bg || "#fff3a3";
      roundRect(ctx, area.x, area.y, area.w, shapeHeight(shape), shape.type === "todo" ? 10 : 8);
      if (ctx.fillStyle !== "transparent") ctx.fill();
      ctx.stroke();
      if (shape.type === "todo") {
        const items = shape.items ?? [];
        const done = items.filter((item) => item.done).length;
        ctx.font = "700 13px 'Segoe UI', Tahoma, sans-serif";
        ctx.fillStyle = shape.color || "#fff";
        ctx.textAlign = "start";
        ctx.fillText(`☑ ${done}/${items.length}`, area.x + 12, area.y + 24);
        ctx.font = "15px 'Segoe UI', Tahoma, sans-serif";
        items.forEach((item, index) => {
          const rowY = area.y + 40 + index * 34 + 17;
          ctx.strokeStyle = shape.color || "#fff";
          ctx.strokeRect(area.x + 12, rowY - 8, 16, 16);
          if (item.done) {
            ctx.fillStyle = "#69db7c";
            ctx.fillRect(area.x + 12, rowY - 8, 16, 16);
          }
          ctx.fillStyle = item.done ? "#8a8a96" : shape.color || "#fff";
          ctx.textAlign = "start";
          ctx.fillText(item.t || "…", area.x + 38, rowY + 5);
          if (item.done && item.t) {
            const w = ctx.measureText(item.t).width;
            ctx.beginPath();
            ctx.moveTo(area.x + 38, rowY + 1);
            ctx.lineTo(area.x + 38 + w, rowY + 1);
            ctx.stroke();
          }
        });
        ctx.textAlign = "center";
      } else if (shape.text) {
        ctx.fillStyle = shape.type === "sticky" ? "#1a1a1a" : shape.color || "#fff";
        ctx.font = "18px 'Segoe UI', Tahoma, sans-serif";
        ctx.textAlign = "center";
        paintText(ctx, shape.text, area.x + area.w / 2, area.y + area.h / 2, Math.max(20, area.w - 24), "center");
      }
      ctx.restore();
      return;
    }

    if (shape.type === "ellipse" || shape.type === "mind") {
      ctx.beginPath();
      ctx.ellipse(area.x + area.w / 2, area.y + area.h / 2, area.w / 2, shapeHeight(shape) / 2, 0, 0, Math.PI * 2);
      if (ctx.fillStyle !== "transparent") ctx.fill();
      ctx.stroke();
      if (shape.text) {
        ctx.fillStyle = shape.color || "#fff";
        ctx.font = "18px 'Segoe UI', Tahoma, sans-serif";
        ctx.textAlign = "center";
        paintText(ctx, shape.text, area.x + area.w / 2, area.y + shapeHeight(shape) / 2, Math.max(20, area.w - 28), "center");
      }
      ctx.restore();
      return;
    }

    if (shape.type === "diamond") {
      const cx = area.x + area.w / 2;
      const cy = area.y + area.h / 2;
      ctx.beginPath();
      ctx.moveTo(cx, area.y);
      ctx.lineTo(area.x + area.w, cy);
      ctx.lineTo(cx, area.y + area.h);
      ctx.lineTo(area.x, cy);
      ctx.closePath();
      if (ctx.fillStyle !== "transparent") ctx.fill();
      ctx.stroke();
      if (shape.text) {
        ctx.fillStyle = shape.color || "#fff";
        ctx.font = "18px 'Segoe UI', Tahoma, sans-serif";
        paintText(ctx, shape.text, cx, cy, Math.max(20, area.w - 30), "center");
      }
      ctx.restore();
      return;
    }

    if (shape.type === "text") {
      ctx.fillStyle = shape.color || "#fff";
      ctx.font = "20px 'Segoe UI', Tahoma, sans-serif";
      ctx.textBaseline = "middle";
      paintText(ctx, shape.text ?? "", shape.x, shape.y + 10, 620, "start");
    }

    ctx.restore();
  }

  // ---------------------------------------------------------------- editing

  async function openEditor(shape: Shape | null, point: Point, multiline: boolean) {
    if (!canvas) return;
    const screen = shape ? toScreen(box(shape).x, box(shape).y) : toScreen(point.x, point.y);
    const width = shape ? Math.max(120, box(shape).w * camera.z) : 260;
    const height = shape ? Math.max(36, (shape.type === "todo" ? shapeHeight(shape) : box(shape).h) * camera.z) : 46;
    let value = shape?.text ?? "";
    if (shape?.type === "todo") value = (shape.items ?? []).map((item) => item.t).join("\n");
    editor = {
      left: screen.x + (shape ? 2 : 0),
      top: screen.y + (shape ? 2 : 0),
      width,
      height,
      value,
      shapeId: shape?.id ?? null,
      multiline,
    };
    await tick();
    editorEl?.focus();
    editorEl?.select();
  }

  function commitEditor() {
    const current = editor;
    if (!current) return;
    editor = null;
    const text = current.value.replace(/\s+$/, "");
    if (current.shapeId === null) {
      if (!text.trim()) {
        draw();
        return;
      }
      snapshot();
      const shape: Shape = {
        id: seq++,
        type: tool === "text" ? "text" : "sticky",
        x: (current.left - camera.x) / camera.z,
        y: (current.top - camera.y) / camera.z,
        x2: (current.left - camera.x) / camera.z + (tool === "text" ? 240 : 240),
        y2: (current.top - camera.y) / camera.z + (tool === "text" ? 30 : 140),
        color: tool === "sticky" ? "#1a1a1a" : stroke,
        bg: tool === "sticky" ? "#fff3a3" : "transparent",
        sw: strokeWidth,
        text,
      };
      shapes = [...shapes, shape];
      selection = new Set([shape.id]);
    } else {
      snapshot();
      shapes = shapes.map((shape) => {
        if (shape.id !== current.shapeId) return shape;
        if (shape.type === "todo") {
          const previous = shape.items ?? [];
          const lines = current.value.split("\n");
          return {
            ...shape,
            items: lines.map((line, index) => ({ t: line.trim(), done: previous[index]?.done ?? false })),
          };
        }
        return { ...shape, text: current.value };
      });
    }
    scheduleSave();
    draw();
  }

  function cancelEditor() {
    editor = null;
    draw();
  }

  // ------------------------------------------------------------------ input

  function onPointerDown(event: PointerEvent) {
    if (editor) commitEditor();
    canvas?.setPointerCapture(event.pointerId);
    const point = toWorld(event.clientX, event.clientY);
    closeMenus();

    if (event.button === 1 || tool === "hand" || spaceHeld) {
      panning = { sx: event.clientX, sy: event.clientY, ox: camera.x, oy: camera.y };
      return;
    }

    if (tool === "select") {
      const hit = hitTest(point);
      if (hit) {
        if (!event.shiftKey && !selection.has(hit.id)) selection = new Set([hit.id]);
        else selection = new Set([...selection, hit.id]);
        dragging = { sx: point.x, sy: point.y, moved: false, before: JSON.stringify(shapes) };
      } else {
        marquee = { x: point.x, y: point.y, x2: point.x, y2: point.y };
        if (!event.shiftKey) selection = new Set();
      }
      draw();
      return;
    }

    if (tool === "eraser") {
      const hit = hitTest(point);
      if (hit) {
        snapshot();
        shapes = shapes.filter((shape) => shape.id !== hit.id && !(shape.type === "mindedge" && (shape.from === hit.id || shape.to === hit.id)));
        scheduleSave();
        draw();
      }
      return;
    }

    if (tool === "text" || tool === "sticky" || tool === "todo" || tool === "mind") {
      if (tool === "mind") {
        snapshot();
        const node: Shape = {
          id: seq++,
          type: "mind",
          x: point.x - 70,
          y: point.y - 40,
          x2: point.x + 70,
          y2: point.y + 40,
          color: stroke,
          bg: fill,
          sw: strokeWidth,
          text: "",
        };
        shapes = [...shapes, node];
        selection = new Set([node.id]);
        scheduleSave();
        draw();
        void openEditor(node, point, true);
        return;
      }
      if (tool === "todo") {
        snapshot();
        const node: Shape = {
          id: seq++,
          type: "todo",
          x: point.x,
          y: point.y,
          x2: point.x + 260,
          y2: point.y + 142,
          color: stroke,
          bg: fill,
          sw: strokeWidth,
          items: [
            { t: "", done: false },
            { t: "", done: false },
            { t: "", done: false },
          ],
        };
        shapes = [...shapes, node];
        selection = new Set([node.id]);
        scheduleSave();
        draw();
        void openEditor(node, point, true);
        return;
      }
      void openEditor(null, point, tool === "sticky");
      return;
    }

    draft = {
      id: seq++,
      type: tool,
      x: point.x,
      y: point.y,
      x2: point.x + 1,
      y2: point.y + 1,
      color: stroke,
      bg: fill,
      sw: strokeWidth,
      points: tool === "draw" ? [point] : undefined,
    };
    draw();
  }

  function onPointerMove(event: PointerEvent) {
    const point = toWorld(event.clientX, event.clientY);
    if (panning) {
      camera = {
        ...camera,
        x: panning.ox + (event.clientX - panning.sx),
        y: panning.oy + (event.clientY - panning.sy),
      };
      draw();
      return;
    }
    if (marquee) {
      marquee = { ...marquee, x2: point.x, y2: point.y };
      draw();
      return;
    }
    if (dragging) {
      const dx = point.x - dragging.sx;
      const dy = point.y - dragging.sy;
      if (Math.abs(dx) + Math.abs(dy) > 1) dragging.moved = true;
      if (dragging.moved) {
        const moved: Shape[] = [];
        for (const shape of shapes) {
          if (!selection.has(shape.id)) {
            moved.push(shape);
            continue;
          }
          const copy: Shape = { ...shape, x: shape.x + dx, y: shape.y + dy, x2: shape.x2 + dx, y2: shape.y2 + dy };
          if (shape.points) copy.points = shape.points.map((p) => ({ x: p.x + dx, y: p.y + dy }));
          moved.push(copy);
        }
        shapes = moved;
        dragging.sx = point.x;
        dragging.sy = point.y;
        draw();
      }
      return;
    }
    if (draft) {
      if (draft.type === "draw") draft.points = [...(draft.points ?? []), point];
      else {
        draft.x2 = point.x;
        draft.y2 = point.y;
      }
      draw();
    }
  }

  function onPointerUp() {
    if (panning) {
      panning = null;
      scheduleSave();
      return;
    }
    if (marquee) {
      const area = {
        x: Math.min(marquee.x, marquee.x2),
        y: Math.min(marquee.y, marquee.y2),
        w: Math.abs(marquee.x2 - marquee.x),
        h: Math.abs(marquee.y2 - marquee.y),
      };
      if (area.w > 4 || area.h > 4) {
        const inside = shapes.filter((shape) => {
          if (shape.type === "mindedge") return false;
          const item = box(shape);
          return item.x < area.x + area.w && item.x + item.w > area.x && item.y < area.y + area.h && item.y + item.h > area.y;
        });
        selection = new Set([...selection, ...inside.map((shape) => shape.id)]);
      }
      marquee = null;
      draw();
      return;
    }
    if (dragging) {
      if (dragging.moved) {
        undoStack.push(dragging.before);
        if (undoStack.length > HISTORY_LIMIT) undoStack.shift();
        redoStack = [];
        scheduleSave();
      }
      dragging = null;
      draw();
      return;
    }
    if (draft) {
      const created = draft;
      draft = null;
      const tiny = Math.abs(created.x2 - created.x) < 4 && Math.abs(created.y2 - created.y) < 4;
      if (created.type !== "draw" && tiny) {
        if (created.type === "rect" || created.type === "ellipse" || created.type === "diamond") {
          created.x2 = created.x + 200;
          created.y2 = created.y + 120;
        }
      }
      if (created.type === "draw" && (created.points?.length ?? 0) < 2) {
        draw();
        return;
      }
      snapshot();
      shapes = [...shapes, created];
      selection = new Set([created.id]);
      scheduleSave();
      draw();
    }
  }

  async function onDoubleClick(event: MouseEvent) {
    const point = toWorld(event.clientX, event.clientY);
    const hit = hitTest(point);
    if (!hit) return;
    selection = new Set([hit.id]);
    draw();
    await openEditor(hit, point, true);
  }

  function onClick(event: MouseEvent) {
    const point = toWorld(event.clientX, event.clientY);
    const hit = hitTest(point);
    if (!hit || hit.type !== "todo") return;
    const area = box(hit);
    const row = Math.floor((point.y - (area.y + 40)) / 34);
    const items = hit.items ?? [];
    if (row >= 0 && row < items.length && point.x >= area.x + 8 && point.x <= area.x + 34) {
      snapshot();
      shapes = shapes.map((shape) =>
        shape.id === hit.id
          ? { ...shape, items: (shape.items ?? []).map((item, index) => (index === row ? { ...item, done: !item.done } : item)) }
          : shape,
      );
      scheduleSave();
      draw();
    }
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.1 : 1 / 1.1;
    zoomAt(event.clientX, event.clientY, factor);
  }

  function zoomAt(clientX: number, clientY: number, factor: number) {
    const before = toWorld(clientX, clientY);
    camera.z = Math.min(4, Math.max(0.2, camera.z * factor));
    const rect = canvas?.getBoundingClientRect();
    camera.x = clientX - (rect?.left ?? 0) - before.x * camera.z;
    camera.y = clientY - (rect?.top ?? 0) - before.y * camera.z;
    draw();
  }

  function zoomBy(factor: number) {
    const rect = canvas?.getBoundingClientRect();
    zoomAt((rect?.left ?? 0) + (rect?.width ?? 0) / 2, (rect?.top ?? 0) + (rect?.height ?? 0) / 2, factor);
  }

  // ---------------------------------------------------------------- actions

  function setTool(next: Tool) {
    tool = next;
    if (next !== "select") selection = new Set();
    draw();
  }

  function deleteSelection() {
    if (selection.size === 0) return;
    snapshot();
    shapes = shapes.filter(
      (shape) =>
        !selection.has(shape.id) &&
        !(shape.type === "mindedge" && (selection.has(shape.from ?? -1) || selection.has(shape.to ?? -1))),
    );
    selection = new Set();
    scheduleSave();
    draw();
  }

  function duplicateSelection() {
    if (selection.size === 0) return;
    snapshot();
    const idMap = new Map<number, number>();
    const clones: Shape[] = [];
    for (const shape of shapes) {
      if (!selection.has(shape.id)) continue;
      const copy: Shape = JSON.parse(JSON.stringify(shape)) as Shape;
      copy.id = seq++;
      idMap.set(shape.id, copy.id);
      copy.x += 24;
      copy.y += 24;
      copy.x2 += 24;
      copy.y2 += 24;
      clones.push(copy);
    }
    for (const clone of clones) {
      if (clone.type === "mindedge" && clone.from !== undefined && clone.to !== undefined) {
        clone.from = idMap.get(clone.from) ?? clone.from;
        clone.to = idMap.get(clone.to) ?? clone.to;
      }
    }
    shapes = [...shapes, ...clones];
    selection = new Set(clones.map((shape) => shape.id));
    scheduleSave();
    draw();
  }

  function addMindChild() {
    const parent = selectedShape;
    if (!parent || parent.type !== "mind") return;
    snapshot();
    const area = box(parent);
    const child: Shape = {
      id: seq++,
      type: "mind",
      x: area.x + area.w + 110,
      y: area.y + area.h / 2 - 35,
      x2: area.x + area.w + 250,
      y2: area.y + area.h / 2 + 35,
      color: parent.color,
      bg: parent.bg,
      sw: parent.sw,
      text: "",
    };
    shapes = [
      ...shapes,
      child,
      { id: seq++, type: "mindedge", x: 0, y: 0, x2: 0, y2: 0, color: parent.color, bg: "transparent", sw: 2, from: parent.id, to: child.id },
    ];
    selection = new Set([child.id]);
    scheduleSave();
    draw();
    void openEditor(child, { x: child.x, y: child.y }, true);
  }

  function clearBoard() {
    if (!confirm("مسح كل ما في السبورة؟")) return;
    snapshot();
    shapes = [];
    selection = new Set();
    scheduleSave();
    draw();
  }

  function contentBox() {
    const visible = shapes.filter((shape) => shape.type !== "mindedge");
    if (visible.length === 0) return null;
    let x1 = Infinity;
    let y1 = Infinity;
    let x2 = -Infinity;
    let y2 = -Infinity;
    for (const shape of visible) {
      const area = box(shape);
      x1 = Math.min(x1, area.x);
      y1 = Math.min(y1, area.y);
      x2 = Math.max(x2, area.x + area.w);
      y2 = Math.max(y2, area.y + shapeHeight(shape));
    }
    return { x: x1 - 30, y: y1 - 30, w: x2 - x1 + 60, h: y2 - y1 + 60 };
  }

  function renderOffscreen(): HTMLCanvasElement {
    const area = contentBox();
    const width = Math.max(480, Math.round(area?.w ?? 900));
    const height = Math.max(320, Math.round(area?.h ?? 600));
    const scale = 2;
    const off = document.createElement("canvas");
    off.width = width * scale;
    off.height = height * scale;
    const ctx = off.getContext("2d");
    if (!ctx) return off;
    ctx.scale(scale, scale);
    ctx.fillStyle = background;
    ctx.fillRect(0, 0, width, height);
    ctx.save();
    ctx.translate(-(area?.x ?? 0), -(area?.y ?? 0));
    for (const shape of shapes) paint(ctx, shape);
    ctx.restore();
    return off;
  }

  function exportPng() {
    const off = renderOffscreen();
    const link = document.createElement("a");
    link.download = "noterust-board.png";
    link.href = off.toDataURL("image/png");
    link.click();
    store.toast("تم تصدير السبورة كصورة PNG");
  }

  /** One-page PDF that embeds the board as a JPEG, built without libraries. */
  function buildPdf(image: Uint8Array, imageWidth: number, imageHeight: number): Blob {
    const portrait = imageHeight > imageWidth;
    const pageWidth = portrait ? 595.28 : 841.89;
    const pageHeight = portrait ? 841.89 : 595.28;
    const margin = 24;
    const scale = Math.min(
      (pageWidth - margin * 2) / imageWidth,
      (pageHeight - margin * 2) / imageHeight,
    );
    const drawWidth = imageWidth * scale;
    const drawHeight = imageHeight * scale;
    const content =
      `q\n${drawWidth.toFixed(2)} 0 0 ${drawHeight.toFixed(2)} ` +
      `${((pageWidth - drawWidth) / 2).toFixed(2)} ${((pageHeight - drawHeight) / 2).toFixed(2)} cm\n` +
      "/Im0 Do\nQ\n";
    const encoder = new TextEncoder();
    const chunks: Uint8Array[] = [];
    let length = 0;
    const pushText = (text: string) => {
      const bytes = encoder.encode(text);
      chunks.push(bytes);
      length += bytes.length;
    };
    const pushBytes = (bytes: Uint8Array) => {
      chunks.push(bytes);
      length += bytes.length;
    };
    const offsets: number[] = [];
    const startObject = (number: number) => {
      offsets[number] = length;
      pushText(`${number} 0 obj\n`);
    };

    pushText("%PDF-1.4\n");
    startObject(1);
    pushText("<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    startObject(2);
    pushText("<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    startObject(3);
    pushText(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${pageWidth.toFixed(2)} ${pageHeight.toFixed(2)}] ` +
        "/Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    startObject(4);
    const contentBytes = encoder.encode(content);
    pushText(`<< /Length ${contentBytes.length} >>\nstream\n`);
    pushBytes(contentBytes);
    pushText("endstream\nendobj\n");
    startObject(5);
    pushText(
      `<< /Type /XObject /Subtype /Image /Width ${imageWidth} /Height ${imageHeight} ` +
        `/ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${image.length} >>\nstream\n`,
    );
    pushBytes(image);
    pushText("\nendstream\nendobj\n");

    const xrefStart = length;
    pushText(`xref\n0 6\n0000000000 65535 f \n`);
    for (let index = 1; index <= 5; index += 1) {
      pushText(`${String(offsets[index]).padStart(10, "0")} 00000 n \n`);
    }
    pushText(`trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n${xrefStart}\n%%EOF\n`);
    return new Blob(chunks as unknown as BlobPart[], { type: "application/pdf" });
  }

  function exportPdf() {
    const off = renderOffscreen();
    const jpeg = off.toDataURL("image/jpeg", 0.92);
    const binary = atob(jpeg.slice(jpeg.indexOf(",") + 1));
    const image = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) {
      image[index] = binary.charCodeAt(index);
    }
    const link = document.createElement("a");
    link.download = "noterust-board.pdf";
    link.href = URL.createObjectURL(buildPdf(image, off.width, off.height));
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 4000);
    store.toast("تم تصدير السبورة كـ PDF");
  }

  function exportJson() {
    const blob = new Blob([JSON.stringify({ type: "noterust-board", version: 1, shapes }, null, 2)], {
      type: "application/json",
    });
    const link = document.createElement("a");
    link.download = "noterust-board.json";
    link.href = URL.createObjectURL(blob);
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 4000);
  }

  function xml(value: string): string {
    return value.replace(/[&<>"]/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[char] ?? char);
  }

  function exportSvg() {
    const area = contentBox() ?? { x: 0, y: 0, w: 900, h: 600 };
    const width = Math.round(area.w);
    const height = Math.round(area.h);
    const parts: string[] = [
      `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`,
      `<rect width="100%" height="100%" fill="${background}"/>`,
    ];
    const X = (value: number) => Math.round((value - area.x) * 100) / 100;
    const Y = (value: number) => Math.round((value - area.y) * 100) / 100;
    for (const shape of shapes) {
      const a = box(shape);
      const style = `stroke="${xml(shape.color)}" stroke-width="${shape.sw}" fill="${
        shape.bg && shape.bg !== "transparent" ? xml(shape.bg) : "none"
      }" stroke-linecap="round" stroke-linejoin="round"`;
      if (shape.type === "rect" || shape.type === "sticky" || shape.type === "idea" || shape.type === "todo") {
        parts.push(
          `<rect x="${X(a.x)}" y="${Y(a.y)}" width="${a.w}" height="${shapeHeight(shape)}" rx="8" ${style}/>`,
        );
        if (shape.text) {
          parts.push(
            `<text x="${X(a.x + a.w / 2)}" y="${Y(a.y + a.h / 2)}" fill="${xml(shape.color)}" font-size="18" text-anchor="middle" dominant-baseline="middle">${xml(shape.text)}</text>`,
          );
        }
      } else if (shape.type === "ellipse" || shape.type === "mind") {
        parts.push(
          `<ellipse cx="${X(a.x + a.w / 2)}" cy="${Y(a.y + a.h / 2)}" rx="${a.w / 2}" ry="${shapeHeight(shape) / 2}" ${style}/>`,
        );
        if (shape.text) {
          parts.push(
            `<text x="${X(a.x + a.w / 2)}" y="${Y(a.y + a.h / 2)}" fill="${xml(shape.color)}" font-size="18" text-anchor="middle" dominant-baseline="middle">${xml(shape.text)}</text>`,
          );
        }
      } else if (shape.type === "diamond") {
        const cx = a.x + a.w / 2;
        const cy = a.y + a.h / 2;
        parts.push(
          `<polygon points="${X(cx)},${Y(a.y)} ${X(a.x + a.w)},${Y(cy)} ${X(cx)},${Y(a.y + a.h)} ${X(a.x)},${Y(cy)}" ${style}/>`,
        );
      } else if (shape.type === "line" || shape.type === "arrow") {
        parts.push(`<line x1="${X(shape.x)}" y1="${Y(shape.y)}" x2="${X(shape.x2)}" y2="${Y(shape.y2)}" ${style}/>`);
      } else if (shape.type === "draw" && shape.points?.length) {
        parts.push(
          `<path d="M ${shape.points.map((p) => `${X(p.x)} ${Y(p.y)}`).join(" L ")}" fill="none" ${style}/>`,
        );
      } else if (shape.type === "text") {
        parts.push(
          `<text x="${X(shape.x)}" y="${Y(shape.y + 10)}" fill="${xml(shape.color)}" font-size="20">${xml(shape.text ?? "")}</text>`,
        );
      }
    }
    parts.push("</svg>");
    const blob = new Blob([parts.join("")], { type: "image/svg+xml" });
    const link = document.createElement("a");
    link.download = "noterust-board.svg";
    link.href = URL.createObjectURL(blob);
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 4000);
  }

  function importJson(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const data = JSON.parse(String(reader.result)) as { shapes?: Shape[] } | Shape[];
        const list = Array.isArray(data) ? data : (data.shapes ?? []);
        if (!Array.isArray(list)) throw new Error("bad file");
        snapshot();
        shapes = list.map((shape) => ({ ...shape, id: seq++ }));
        selection = new Set();
        scheduleSave();
        draw();
        store.toast("تم تحميل السبورة");
      } catch {
        store.toast("ملف السبورة غير صالح", "error");
      }
    };
    reader.readAsText(file);
  }

  // ------------------------------------------------------------------ keys

  function onKeydown(event: KeyboardEvent) {
    if (get(store.view) !== "board") return;
    const target = event.target as HTMLElement | null;
    if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
    const key = event.key.toLowerCase();
    const ctrl = event.ctrlKey || event.metaKey;

    if (event.key === " ") {
      spaceHeld = true;
      event.preventDefault();
      return;
    }
    if (ctrl && key === "z") {
      event.preventDefault();
      event.shiftKey ? redo() : undo();
      return;
    }
    if (ctrl && key === "y") {
      event.preventDefault();
      redo();
      return;
    }
    if (ctrl && key === "a") {
      event.preventDefault();
      selection = new Set(shapes.filter((s) => s.type !== "mindedge").map((s) => s.id));
      draw();
      return;
    }
    if (ctrl && key === "d") {
      event.preventDefault();
      duplicateSelection();
      return;
    }
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      deleteSelection();
      return;
    }
    if (ctrl) return;

    const map: Record<string, Tool> = {
      v: "select",
      h: "hand",
      r: "rect",
      o: "ellipse",
      d: "diamond",
      a: "arrow",
      l: "line",
      p: "draw",
      t: "text",
      n: "sticky",
      u: "todo",
      f: "idea",
      m: "mind",
      e: "eraser",
    };
    if (map[key]) {
      event.preventDefault();
      setTool(map[key]);
    }
  }

  function onKeyup(event: KeyboardEvent) {
    if (event.key === " ") spaceHeld = false;
  }

  function closeMenus() {
    /* reserved for popovers */
  }
</script>

<section class="view-page">
  <header class="page-head">
    <BackButton />
    <div>
      <h1>السبورة</h1>
      <p class="muted">ارسم مخططات وملاحظات لاصقة وخرائط ذهنية، وتُحفظ تلقائياً.</p>
    </div>
    <span class="badge">{plural(count, NOUNS.snippet)}</span>
  </header>

  <div class="tools">
    <div class="tool-group">
      {#each TOOLS as [toolName, glyph, label] (toolName)}
        <button class="tool" class:on={tool === toolName} title={label} on:click={() => setTool(toolName)}>
          {glyph}
        </button>
      {/each}
    </div>

    <span class="divider"></span>

    <div class="tool-group">
      <button class="tool" class:on={canUndo} title="تراجع (Ctrl+Z)" on:click={undo} disabled={!canUndo}>
        ↩
      </button>
      <button class="tool" class:on={canRedo} title="إعادة (Ctrl+Y)" on:click={redo} disabled={!canRedo}>
        ↪
      </button>
      <button class="tool" title="تكرار (Ctrl+D)" on:click={duplicateSelection} disabled={selection.size === 0}>
        ⧉
      </button>
      <button class="tool" title="حذف (Delete)" on:click={deleteSelection} disabled={selection.size === 0}>
        🗑
      </button>
      {#if selectedShape?.type === "mind"}
        <button class="tool wide" title="إضافة فرع للعقدة" on:click={addMindChild}>+ فرع</button>
      {/if}
    </div>

    <span class="divider"></span>

    <div class="tool-group colors">
      {#each HSV as color (color)}
        <button
          class="swatch"
          class:on={stroke === color}
          style="background:{color}"
          title={color}
          on:click={() => (stroke = color)}
        ></button>
      {/each}
    </div>

    <div class="tool-group">
      <span class="label">تعبئة</span>
      {#each FILLS as color (color)}
        <button
          class="swatch"
          class:on={fill === color}
          class:transparent={color === "transparent"}
          style={color === "transparent" ? "" : `background:${color}`}
          title={color}
          on:click={() => (fill = color)}
        ></button>
      {/each}
    </div>

    <div class="tool-group">
      <span class="label">سُمك</span>
      <input type="range" min="1" max="12" step="0.5" bind:value={strokeWidth} title="سُمك الخط" />
    </div>

    <span class="divider"></span>

    <div class="tool-group">
      <button class="tool" title="تصغير" on:click={() => zoomBy(1 / 1.15)}>−</button>
      <span class="zoom">{arabicDigits(Math.round(camera.z * 100))}%</span>
      <button class="tool" title="تكبير" on:click={() => zoomBy(1.15)}>+</button>
      <button class="tool" class:on={grid} title="الشبكة" on:click={() => { grid = !grid; scheduleSave(); draw(); }}>▦</button>
    </div>

    <span class="spacer"></span>

    <div class="tool-group">
      <span class="save-state" title="يُحفظ كل تعديل تلقائياً على القرص">
        {#if saving}جارٍ الحفظ…{:else if savedAt}محفوظ تلقائياً{:else}—{/if}
      </span>
      <button class="btn small" title="تصدير PNG" on:click={exportPng}>PNG</button>
      <button class="btn small" title="تصدير PDF" on:click={exportPdf}>PDF</button>
      <button class="btn small" title="تصدير SVG" on:click={exportSvg}>SVG</button>
      <button class="btn small" on:click={exportJson}>حفظ JSON</button>
      <button class="btn small" on:click={() => fileInput?.click()}>فتح JSON</button>
      <button class="btn small ghost" on:click={clearBoard}>مسح</button>
    </div>
    <input class="hidden-input" type="file" accept=".json" bind:this={fileInput} on:change={importJson} />
  </div>

  <div class="canvas-wrap" bind:this={host}>
    <canvas
      bind:this={canvas}
      class:pan={tool === "hand" || spaceHeld}
      on:pointerdown={onPointerDown}
      on:pointermove={onPointerMove}
      on:pointerup={onPointerUp}
      on:pointercancel={onPointerUp}
      on:dblclick={onDoubleClick}
      on:click={onClick}
      on:wheel={onWheel}
    ></canvas>

    {#if editor}
      <textarea
        bind:this={editorEl}
        class="text-editor"
        style="left:{editor.left}px; top:{editor.top}px; width:{editor.width}px; height:{editor.height}px"
        bind:value={editor.value}
        dir="auto"
        on:blur={commitEditor}
        on:keydown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            cancelEditor();
          } else if (event.key === "Enter" && (event.ctrlKey || event.metaKey || !editor?.multiline)) {
            event.preventDefault();
            commitEditor();
          }
        }}
      ></textarea>
    {/if}

    <div class="hint">
      عجلة الفأرة = تكبير · مسافة + سحب = تحريك · دبل كليك على أي عنصر = تحرير النص
    </div>
  </div>
</section>

<style>
  .view-page {
    grid-column: 3 / -1;
    grid-row: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
  }

  .page-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 26px 12px;
    border-bottom: 1px solid var(--border);
  }

  h1 {
    margin: 0;
    font-size: 1.24em;
  }

  .page-head p {
    margin: 2px 0 0;
    font-size: 0.8em;
  }

  .badge {
    margin-inline-start: auto;
    font-size: 0.74em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 3px 11px;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    padding: 8px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .tool-group {
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 3px 5px;
  }

  .tool {
    min-width: 30px;
    height: 30px;
    padding: 0 6px;
    border-radius: 7px;
    color: var(--text-muted);
    font-size: 0.9em;
    display: grid;
    place-items: center;
  }

  .tool:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .tool.on {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .tool:disabled {
    opacity: 0.35;
  }

  .tool.wide {
    font-size: 0.76em;
    padding: 0 9px;
  }

  .divider {
    width: 1px;
    height: 24px;
    background: var(--border);
  }

  .label {
    font-size: 0.68em;
    color: var(--text-faint);
    padding: 0 4px;
  }

  .colors .swatch,
  .swatch {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    border: 2px solid transparent;
  }

  .swatch.on {
    border-color: var(--text);
  }

  .swatch.transparent {
    background: repeating-conic-gradient(#666 0 25%, #fff 0 50%) 0 0 / 8px 8px;
  }

  input[type="range"] {
    width: 80px;
    accent-color: var(--accent);
  }

  .zoom {
    font-size: 0.72em;
    color: var(--text-muted);
    min-width: 38px;
    text-align: center;
  }

  .btn.small {
    font-size: 0.74em;
    padding: 4px 9px;
  }

  .spacer {
    flex: 1;
  }

  .save-state {
    font-size: 0.72em;
    color: var(--text-faint);
    min-width: 84px;
    text-align: center;
    user-select: none;
  }

  .hidden-input {
    display: none;
  }

  .canvas-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: crosshair;
    touch-action: none;
  }

  canvas.pan {
    cursor: grab;
  }

  .text-editor {
    position: absolute;
    background: rgba(20, 20, 28, 0.92);
    border: 1.5px dashed var(--accent);
    border-radius: 8px;
    color: var(--text);
    padding: 8px;
    font-family: inherit;
    font-size: 0.9em;
    line-height: 1.6;
    resize: none;
    outline: none;
    z-index: 5;
  }

  .hint {
    position: absolute;
    bottom: 10px;
    inset-inline-start: 14px;
    font-size: 0.7em;
    color: var(--text-faint);
    background: rgba(20, 20, 28, 0.75);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 4px 12px;
    pointer-events: none;
  }
</style>
