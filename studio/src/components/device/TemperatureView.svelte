<script lang="ts">
  // A thermo-optic shifter's temperature on its cross-section, by its 2D model at the design's
  // power, or at the power for a π shift while the design's is zero
  // (studio/src-tauri/src/devices/fields.rs): the rise above the ambient from black to pale
  // yellow, its isotherms, the media's edges, and where the guide's mode takes its phase from the
  // heat (its thermo-optic weights at half and 1/e² of their largest); with the drive per unit
  // length, the peak, and the rise the mode feels. Whatever model the page uses, the field is the
  // 2D model's: the analytic one gives the rise at the guide's centre only.
  import { api, type DeviceSpec, type Temperature } from "../../lib/api";
  import { range } from "../../lib/colours";
  import { plain } from "../../lib/deviceview";
  import RasterView from "../RasterView.svelte";

  let { spec, height = 300 }: { spec: DeviceSpec; height?: number } = $props();

  /** The design at the 2D model: the temperature doesn't change with the model the page shows. */
  const asked = $derived(JSON.stringify({ ...spec, fidelity: "2D", compact: undefined }));
  let shown = $state<Temperature | null>(null);
  let problem = $state("");
  let busy = $state(false);
  $effect(() => {
    const s = JSON.parse(asked) as DeviceSpec;
    let current = true;
    busy = true;
    const timer = setTimeout(() => {
      api
        .deviceTemperature(s)
        .then((t) => {
          if (!current) return;
          shown = t;
          problem = "";
        })
        .catch((e) => {
          if (current) problem = String(e);
        })
        .finally(() => {
          if (current) busy = false;
        });
    }, 300);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  });

  const kelvin = (v: number) => `${Number(v.toPrecision(3))} K`;
  const milliwatts = (v: number) => `${Number(v.toPrecision(4))} mW`;
  const ends = $derived(shown ? range(shown.raster) : [0, 0]);
  const step = $derived(shown && shown.isotherms.length > 1 ? shown.isotherms[1].level - shown.isotherms[0].level : (shown?.isotherms[0]?.level ?? 0));
  const lines = $derived(
    shown
      ? [
          { lines: shown.isotherms.flatMap((i) => i.lines), tone: "level" as const },
          { lines: shown.mode[0]?.lines ?? [], tone: "mode" as const },
          { lines: shown.mode[1]?.lines ?? [], tone: "mode-faint" as const },
        ]
      : [],
  );
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
  {#if problem}
    <p class="text-sm text-warning selectable">{plain(problem)}</p>
  {:else if shown}
    <p class="text-xs muted">
      The 2D model's temperature rise
      {#if shown.at_pi}at the power for a π shift, <span class="num">{milliwatts(shown.p_pi_mw)}</span> (the design's power is zero){:else}at the design's power, <span class="num"
          >{milliwatts(shown.power_mw)}</span
        > (a π shift takes <span class="num">{milliwatts(shown.p_pi_mw)}</span>){/if}: <span class="num">{Number(shown.watts_per_metre.toPrecision(4))} W/m</span> along the guide, at most
      <span class="num">{kelvin(shown.peak)}</span>. The mode feels <span class="num">{kelvin(shown.guide)}</span>, the mean of the rise weighted by where it takes its phase from.
      {#if busy}<span class="loading loading-spinner loading-xs align-middle text-primary"></span>{/if}
    </p>
    <RasterView
      raster={shown.raster}
      kind="intensity"
      axes={["x", "y"]}
      maxHeight={Math.max(120, height - 90)}
      quantity="rise (K)"
      outline={{ shapes: shown.outline, interfaces: [] }}
      {lines}
      ends={[kelvin(ends[0]), kelvin(ends[1])]}
    />
    <p class="text-[11px] faint">
      From <span class="num">{kelvin(ends[0])}</span> (black) to <span class="num">{kelvin(ends[1])}</span> (pale yellow) in this window; x across the guide from its centre, y up from the buried
      oxide's top. White: the media's edges{#if step > 0}; thin lines, isotherms every <span class="num">{kelvin(step)}</span>{/if}. Teal: where the mode takes its phase from the heat, its
      thermo-optic weight at half its largest (solid) and at 1/e² (dashed). {shown.grid}.
    </p>
  {:else}
    <div class="skeleton flex-1" aria-busy="true" aria-label="Solving the temperature"></div>
  {/if}
</div>
