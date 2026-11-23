<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import Modal from "./Modal.svelte";
  import type { Note } from "../lib/types";

  export let note: Note;
  export let mode: "lock" | "remove" = "lock";

  const dispatch = createEventDispatcher<{ close: void; saved: Note }>();

  let password = "";
  let confirmPassword = "";
  let busy = false;
  let error = "";

  async function submit() {
    if (!password) {
      error = "اكتب كلمة المرور";
      return;
    }
    if (mode === "lock" && password !== confirmPassword) {
      error = "كلمتا المرور غير متطابقتين";
      return;
    }
    busy = true;
    error = "";
    try {
      const updated =
        mode === "lock"
          ? await api.lockNote(note.id, password)
          : await api.removeLock(note.id, password);
      store.toast(mode === "lock" ? "تم قفل الملاحظة" : "تمت إزالة القفل");
      dispatch("saved", updated);
      dispatch("close");
    } catch (err) {
      error = store.errorMessage(err);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  title={mode === "lock" ? "قفل الملاحظة بكلمة مرور" : "إزالة القفل"}
  on:close={() => dispatch("close")}
>
  <p class="hint">
    {#if mode === "lock"}
      يُشفَّر نص الملاحظة محلياً بـ AES-256-GCM ومفتاح مشتق بـ Argon2. لا يمكن استرجاع
      كلمة المرور إن نسيتها.
    {:else}
      أدخل كلمة المرور الحالية لفك التشفير نهائياً.
    {/if}
  </p>

  <div class="field">
    <label for="lock-password">كلمة المرور</label>
    <input
      id="lock-password"
      type="password"
      bind:value={password}
      autocomplete="off"
      on:keydown={(event) => event.key === "Enter" && submit()}
    />
  </div>

  {#if mode === "lock"}
    <div class="field">
      <label for="lock-confirm">تأكيد كلمة المرور</label>
      <input
        id="lock-confirm"
        type="password"
        bind:value={confirmPassword}
        autocomplete="off"
        on:keydown={(event) => event.key === "Enter" && submit()}
      />
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <svelte:fragment slot="footer">
    <button class="btn primary" on:click={submit} disabled={busy}>
      {mode === "lock" ? "قفل" : "إزالة القفل"}
    </button>
    <button class="btn ghost" on:click={() => dispatch("close")}>إلغاء</button>
  </svelte:fragment>
</Modal>

<style>
  .hint {
    margin: 0 0 14px;
    color: var(--text-muted);
    font-size: 0.84em;
    line-height: 1.7;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.82em;
  }
</style>
