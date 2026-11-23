<script lang="ts">
  export interface Segment {
    key: string;
    label: string;
    value: number;
    color: string;
  }

  export let segments: Segment[] = [];
  export let size = 170;
  export let thickness = 22;
  export let centerLabel = "";
  export let centerValue = "";
  export let emptyText = "لا توجد بيانات";

  let hovered: string | null = null;

  $: radius = (size - thickness) / 2;
  $: circumference = 2 * Math.PI * radius;
  $: total = segments.reduce((sum, item) => sum + Math.max(0, item.value), 0);

  interface Piece extends Segment {
    length: number;
    offset: number;
    ratio: number;
  }

  $: pieces = build(segments, total, circumference);

  function build(list: Segment[], sum: number, length: number): Piece[] {
    if (sum <= 0) return [];
    const gap = list.length > 1 ? 3 : 0;
    let cursor = 0;
    return list
      .filter((item) => item.value > 0)
      .map((item) => {
        const ratio = item.value / sum;
        const raw = ratio * length;
        const drawn = Math.max(0, raw - gap);
        const piece: Piece = { ...item, length: drawn, offset: cursor, ratio };
        cursor += raw;
        return piece;
      });
  }
</script>

<div class="donut" style="width:{size}px">
  {#if total > 0}
    <svg viewBox="0 0 {size} {size}" width={size} height={size} role="img" aria-label={centerLabel}>
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke="var(--bg-hover)"
        stroke-width={thickness}
      />
      {#each pieces as piece (piece.key)}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={piece.color}
          stroke-width={hovered === piece.key ? thickness + 3 : thickness}
          stroke-dasharray="{piece.length} {circumference - piece.length}"
          stroke-dashoffset={-piece.offset}
          transform="rotate(-90 {size / 2} {size / 2})"
          role="presentation"
          on:mouseenter={() => (hovered = piece.key)}
          on:mouseleave={() => (hovered = null)}
        >
          <title>{piece.label}: {centerValue && total > 0 ? Math.round(piece.ratio * 100) : 0}%</title>
        </circle>
      {/each}
    </svg>
    <div class="center">
      <span class="center-value">{centerValue}</span>
      <span class="center-label">{centerLabel}</span>
    </div>
  {:else}
    <p class="empty">{emptyText}</p>
  {/if}
</div>

<style>
  .donut {
    position: relative;
    display: grid;
    place-items: center;
    margin-inline: auto;
  }

  svg {
    display: block;
  }

  circle {
    transition: stroke-width 140ms ease;
    cursor: default;
  }

  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1px;
    pointer-events: none;
  }

  .center-value {
    font-size: 0.98em;
    font-weight: 700;
    max-width: 78%;
    text-align: center;
  }

  .center-label {
    font-size: 0.68em;
    color: var(--text-faint);
  }

  .empty {
    margin: 0;
    padding: 40px 8px;
    color: var(--text-faint);
    font-size: 0.8em;
    text-align: center;
  }
</style>
