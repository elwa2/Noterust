<script lang="ts">
  import { iconBody } from "../lib/icons";

  export let name: string;
  export let size = 18;
  export let stroke = 1.6;
  export let filled = false;

  $: body = iconBody(name);
</script>

<svg
  class="ico"
  width={size}
  height={size}
  viewBox="0 0 20 20"
  fill={filled ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false">{@html body}</svg
>

<style>
  .ico {
    display: block;
    flex-shrink: 0;
  }
</style>
