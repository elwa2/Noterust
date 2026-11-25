<script lang="ts">
  import { onMount } from "svelte";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  // -1: لم يبدأ التنزيل، 0..100: النسبة الحالية
  let progress = -1;
  let version = "";
  let ready = false;
  let dismissed = false;
  let restarting = false;
  let pending: Update | null = null;
  let failed = false;

  onMount(() => {
    let cancelled = false;
    void (async () => {
      try {
        const found = await check();
        if (!found || cancelled) return;
        pending = found;
        version = found.version;

        let total = 0;
        let done = 0;
        await found.download((event) => {
          if (cancelled) return;
          if (event.event === "Started") {
            total = event.data.contentLength ?? 0;
            progress = 0;
          } else if (event.event === "Progress") {
            done += event.data.chunkLength;
            progress = total > 0 ? Math.min(100, Math.round((done / total) * 100)) : -1;
          }
        });
        if (!cancelled) ready = true;
      } catch (err) {
        // لا شبكة أو لا إصدار أحدث: لا نزعج المستخدم بشيء.
        failed = true;
        console.warn("فشل التحقق من التحديثات", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  async function restart() {
    if (!pending) return;
    restarting = true;
    try {
      await pending.install();
      await relaunch();
    } catch (err) {
      restarting = false;
      failed = true;
      console.warn("فشل تثبيت التحديث", err);
    }
  }
</script>

{#if ready && !dismissed && !failed}
  <div class="update-card" role="dialog" aria-live="polite" aria-label="تحديث جاهز">
    <div class="head">
      <span class="badge" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="9" />
          <path d="M12 16V8" />
          <path d="m8.5 11.5 3.5-3.5 3.5 3.5" />
        </svg>
      </span>
      <div class="title">
        <strong>الإصدار {version} جاهز</strong>
        <span>سيتم التحديث عند إغلاق التطبيق، أو أعد التشغيل الآن.</span>
      </div>
    </div>

    <div class="actions">
      <button class="primary" on:click={restart} disabled={restarting}>
        {restarting ? "جارٍ التحديث…" : "إعادة التشغيل والتحديث"}
      </button>
      <button class="ghost" on:click={() => (dismissed = true)} disabled={restarting}>لاحقاً</button>
    </div>
  </div>
{/if}

{#if progress >= 0 && !ready && !failed}
  <div class="update-pill">
    <span>يتم تنزيل تحديث {version}…</span>
    <span class="bar"><i style={`width: ${progress}%`}></i></span>
  </div>
{/if}

<style>
  .update-card {
    position: fixed;
    inset-inline-start: 18px;
    bottom: 18px;
    z-index: 95;
    width: min(420px, calc(100vw - 36px));
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    box-shadow: var(--shadow);
    padding: 16px 18px 14px;
    color: var(--text);
    font-size: 0.92em;
  }

  .head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .badge {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    border-radius: 50%;
    border: 1.5px solid var(--accent);
    color: var(--accent);
  }

  .title {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title strong {
    font-size: 1.02em;
    font-weight: 700;
  }

  .title span {
    color: var(--text-muted);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 14px;
  }

  button {
    font-family: inherit;
    font-size: 1em;
    border-radius: 999px;
    padding: 7px 16px;
    cursor: pointer;
    border: 1px solid transparent;
  }

  button:disabled {
    opacity: 0.65;
    cursor: default;
  }

  .primary {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 700;
  }

  .primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }

  .ghost {
    background: transparent;
    color: var(--text-muted);
    border-color: var(--border);
  }

  .ghost:hover:not(:disabled) {
    color: var(--text);
  }

  .update-pill {
    position: fixed;
    inset-inline-start: 18px;
    bottom: 16px;
    z-index: 95;
    display: flex;
    align-items: center;
    gap: 10px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 6px 14px;
    font-size: 0.82em;
    color: var(--text-muted);
    box-shadow: var(--shadow);
  }

  .bar {
    display: block;
    width: 70px;
    height: 4px;
    border-radius: 999px;
    background: var(--bg-hover);
    overflow: hidden;
  }

  .bar i {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 160ms ease;
  }
</style>
