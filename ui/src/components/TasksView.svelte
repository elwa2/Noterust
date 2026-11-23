<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import { arabicDigits, fromInputDate, NOUNS, plural, relativeDay, toInputDate } from "../lib/format";
  import Icon from "./Icon.svelte";
  import type { TaskItem } from "../lib/types";

  let tasks: TaskItem[] = [];
  let loading = true;
  let filter: "open" | "done" | "all" = "open";

  let title = "";
  let due = "";
  let priority = 2;
  let repeat = "";
  let adding = false;
  let error = "";

  $: visible =
    filter === "open"
      ? tasks.filter((task) => !task.isDone)
      : filter === "done"
        ? tasks.filter((task) => task.isDone)
        : tasks;

  $: openCount = tasks.filter((task) => !task.isDone).length;

  onMount(() => void load());

  async function load() {
    loading = true;
    try {
      tasks = await api.listTasks(true);
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    } finally {
      loading = false;
    }
  }

  async function addTask() {
    const trimmed = title.trim();
    if (!trimmed) {
      error = "اكتب عنوان المهمة";
      return;
    }
    adding = true;
    error = "";
    try {
      await api.createTask({
        title: trimmed,
        dueAt: fromInputDate(due),
        priority,
        repeatRule: repeat || null,
      });
      title = "";
      due = "";
      repeat = "";
      priority = 2;
      await load();
      await store.refreshCounts();
      store.toast("تمت إضافة المهمة");
    } catch (err) {
      error = store.errorMessage(err);
    } finally {
      adding = false;
    }
  }

  async function toggle(task: TaskItem) {
    try {
      await api.completeTask(task.id, !task.isDone);
      await load();
      await store.refreshCounts();
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  async function remove(task: TaskItem) {
    if (!confirm(`حذف مهمة «${task.title}»؟`)) return;
    try {
      await api.deleteTask(task.id);
      await load();
      await store.refreshCounts();
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  async function openNote(task: TaskItem) {
    if (!task.noteId) return;
    await store.setView("notes");
    await store.openNote(task.noteId);
  }

  function overdue(task: TaskItem): boolean {
    if (!task.dueAt || task.isDone) return false;
    return task.dueAt < new Date(new Date().toDateString()).getTime();
  }

  function dueLabel(task: TaskItem): string {
    if (!task.dueAt) return "";
    return relativeDay(task.dueAt);
  }

  $: summary = plural(openCount, NOUNS.task);

  const priorityLabels: Record<number, string> = { 1: "مرتفعة", 2: "متوسطة", 3: "منخفضة" };
  const repeatLabels: Record<string, string> = {
    daily: "يومياً",
    weekly: "أسبوعياً",
    monthly: "شهرياً",
    yearly: "سنوياً",
    checklist: "من ملاحظة",
  };
</script>

<section class="view-page">
  <header class="page-head">
    <div>
      <h1>المهام</h1>
      <p class="muted">متابعة كل ما يجب إنجازه، بما فيها قوائم التحقق داخل الملاحظات.</p>
    </div>
    <span class="badge">{summary}</span>
  </header>

  <div class="scroll content">
    <div class="card add-card">
      <div class="add-row">
        <input
          class="grow"
          placeholder="مهمة جديدة…"
          bind:value={title}
          on:keydown={(event) => event.key === "Enter" && addTask()}
        />
        <input type="date" bind:value={due} title="تاريخ الاستحقاق" />
        <select bind:value={priority} title="الأولوية">
          <option value={1}>مرتفعة</option>
          <option value={2}>متوسطة</option>
          <option value={3}>منخفضة</option>
        </select>
        <select bind:value={repeat} title="التكرار">
          <option value="">بلا تكرار</option>
          <option value="daily">يومياً</option>
          <option value="weekly">أسبوعياً</option>
          <option value="monthly">شهرياً</option>
          <option value="yearly">سنوياً</option>
        </select>
        <button class="btn primary" on:click={addTask} disabled={adding}>
          <Icon name="plus" size={14} /> إضافة
        </button>
      </div>
      {#if error}<p class="error">{error}</p>{/if}
    </div>

    <div class="filters">
      <button class="chip" class:on={filter === "open"} on:click={() => (filter = "open")}>غير مكتملة</button>
      <button class="chip" class:on={filter === "done"} on:click={() => (filter = "done")}>مكتملة</button>
      <button class="chip" class:on={filter === "all"} on:click={() => (filter = "all")}>الكل</button>
      <span class="spacer"></span>
      <span class="faint">اليوم: {relativeDay(Date.now())} · {arabicDigits(tasks.length)} إجمالاً</span>
    </div>

    {#if loading}
      <p class="muted">جارٍ التحميل…</p>
    {:else if visible.length === 0}
      <p class="empty">
        {filter === "done" ? "لا مهام مكتملة بعد." : "لا مهام — أضف مهمة أو اكتب «- [ ] شيء» داخل ملاحظة."}
      </p>
    {:else}
      <ul class="task-list">
        {#each visible as task (task.id)}
          <li class="task" class:done={task.isDone} class:overdue={overdue(task)}>
            <button
              class="check"
              class:checked={task.isDone}
              on:click={() => toggle(task)}
              title={task.isDone ? "إعادة فتح" : "إكمال"}
            >
              {task.isDone ? "✓" : ""}
            </button>

            <div class="task-main">
              <div class="task-title">{task.title}</div>
              {#if task.description}
                <p class="desc">{task.description}</p>
              {/if}
              <div class="task-meta">
                <span class="prio p{task.priority}">{priorityLabels[task.priority] ?? "متوسطة"}</span>
                {#if task.dueAt}
                  <span class="due" class:late={overdue(task)}>
                    <Icon name="calendar" size={12} /> {dueLabel(task)}
                  </span>
                {/if}
                {#if task.repeatRule}
                  <span class="tag">{repeatLabels[task.repeatRule] ?? task.repeatRule}</span>
                {/if}
                {#if task.noteTitle}
                  <button class="note-link" on:click={() => openNote(task)}>
                    <Icon name="notes" size={12} /> {task.noteTitle}
                  </button>
                {/if}
              </div>
            </div>

            <button class="icon-btn" title="حذف" on:click={() => remove(task)}>
              <Icon name="trash" size={15} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

<style>
  .view-page {
    grid-column: 2 / -1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg);
  }

  .page-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px 26px 14px;
    border-bottom: 1px solid var(--border);
  }

  h1 {
    margin: 0;
    font-size: 1.28em;
  }

  .page-head p {
    margin: 2px 0 0;
    font-size: 0.82em;
  }

  .content {
    flex: 1;
    padding: 18px 26px 40px;
    overflow-y: auto;
  }

  .card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px;
  }

  .add-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .grow {
    flex: 1;
    min-width: 220px;
  }

  .filters {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 14px 0;
  }

  .chip {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 4px 12px;
    font-size: 0.8em;
    color: var(--text-muted);
    background: var(--bg-elevated);
  }

  .chip.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }

  .task-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .task {
    display: flex;
    align-items: flex-start;
    gap: 11px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 14px;
  }

  .task.overdue {
    border-inline-start: 3px solid var(--danger);
  }

  .task.done .task-title {
    text-decoration: line-through;
    color: var(--text-faint);
  }

  .check {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    display: grid;
    place-items: center;
    font-size: 0.7em;
    margin-top: 3px;
    flex-shrink: 0;
  }

  .check.checked {
    background: var(--success);
    border-color: var(--success);
    color: #12241a;
  }

  .task-main {
    flex: 1;
    min-width: 0;
  }

  .task-title {
    font-size: 0.94em;
    font-weight: 600;
  }

  .desc {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 0.8em;
  }

  .task-meta {
    display: flex;
    align-items: center;
    gap: 9px;
    flex-wrap: wrap;
    margin-top: 5px;
    font-size: 0.73em;
    color: var(--text-faint);
  }

  .prio.p1 {
    color: var(--danger);
  }

  .prio.p2 {
    color: var(--accent);
  }

  .prio.p3 {
    color: var(--text-muted);
  }

  .due {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .due.late {
    color: var(--danger);
  }

  .note-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .tag {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
  }

  .note-link {
    color: var(--accent);
    font-size: 1em;
  }

  .empty {
    color: var(--text-faint);
    text-align: center;
    padding: 36px;
    font-size: 0.88em;
  }

  .error {
    margin: 8px 0 0;
    color: var(--danger);
    font-size: 0.8em;
  }
</style>
