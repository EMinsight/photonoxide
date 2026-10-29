<script lang="ts">
  // A static job's fields on its cross-section, as the run recorded them (photonoxide::job's
  // StaticField): each a picture from its smallest value (black) to its largest (pale yellow),
  // the structure's edges over it from the run's scene; a temperature with its isotherms; and
  // with an electro_optic table, the electrode-field view: the electrodes' |E| on the mode's
  // window (its colours up to where 99% of its pixels are, past the corners' singular field)
  // with the mode's outline over it (its |E|² at half and 1/e² of its peak), where the two
  // overlap being what moves the mode's index.
  import { api, type Isoline } from "../lib/api";
  import { quantile, range } from "../lib/colours";
  import type { Scene, StaticField } from "../lib/events";
  import { cutOutline } from "../lib/outline";
  import { label as tick, ticks } from "../lib/plot";
  import RasterView from "./RasterView.svelte";

  let { statics, scene, outlined = true }: { statics: StaticField[]; scene: Scene | null; outlined?: boolean } = $props();

  const FIELD = "electric field |E|";
  const MODE = "mode's |E|^2";
  /** The share of |E|'s pixels its colours span: the electrodes' corners are singular, and would leave the gap dark. */
  const CAP = 0.99;
  const electrode = $derived(statics.find((s) => s.label === FIELD) ?? null);
  const mode = $derived(statics.find((s) => s.label === MODE) ?? null);
  /** The fields drawn each on its own: all but the two the electrode-field view draws together. */
  const own = $derived(electrode && mode ? statics.filter((s) => s !== electrode && s !== mode) : statics);

  /** The structure's edges on a field's picture: the scene cut where the job cuts it, the middle of the scene's length along x. */
  const outline = (s: StaticField) => (outlined && scene ? cutOutline(scene, "x", (scene.x_um[0] + scene.x_um[1]) / 2, [s.raster.x0, s.raster.x1]) : undefined);
  const heat = (s: StaticField) => s.label.startsWith("temperature");
  /** Round levels strictly inside a field's range: its isotherms. */
  const levels = (s: StaticField) => {
    const [lo, hi] = range(s.raster);
    return ticks(lo, hi, 8).filter((t) => t > lo && t < hi);
  };

  // the lines over the pictures, from the program: each temperature's isotherms, the mode's outline
  let isotherms = $state<Isoline[][]>([]);
  let outlineOfMode = $state<Isoline[]>([]);
  $effect(() => {
    const list = $state.snapshot(own) as StaticField[];
    let current = true;
    Promise.all(list.map((s) => (heat(s) ? api.rasterContours(s.raster, levels(s)).catch(() => []) : Promise.resolve([])))).then((l) => {
      if (current) isotherms = l;
    });
    return () => (current = false);
  });
  $effect(() => {
    const m = mode ? ($state.snapshot(mode) as StaticField) : null;
    let current = true;
    if (!m) {
      outlineOfMode = [];
      return;
    }
    api
      .rasterContours(m.raster, [0.5, Math.exp(-2)])
      .then((l) => {
        if (current) outlineOfMode = l;
      })
      .catch(() => {});
    return () => (current = false);
  });
  const fmt = (v: number) => v.toPrecision(4);
  /** A colour bar's end: two digits. */
  const short = (v: number) => (Math.abs(v) >= 1e4 ? v.toExponential(1) : tick(v));
  const step = (s: StaticField) => {
    const l = levels(s);
    return l.length > 1 ? l[1] - l[0] : 0;
  };
</script>

{#if electrode && mode}
  {@const [lo, hi] = range(electrode.raster)}
  {@const top = Math.min(hi, quantile(electrode.raster, CAP))}
  <section class="panel p-5">
    <h3 class="mb-1 font-semibold">The electrodes' field and the mode <span class="font-normal faint">· |E| in V/m, on the mode's window</span></h3>
    <p class="mb-3 text-xs faint">
      The electrodes' |E| at their potentials, from <span class="num">{fmt(lo)}</span> V/m (black) to <span class="num">{fmt(top)}</span> V/m (pale yellow){#if top < hi}, where 99% of the pixels are: the
        electrodes' corners reach <span class="num">{fmt(hi)}</span> V/m{/if}; zero inside a conductor. Teal, the optical mode's |E|² at half its peak (solid) and at 1/e² (dashed). Where the mode sits in a strong
      field the crystal's index moves most: that overlap sets Vπ·L.
    </p>
    <RasterView
      raster={electrode.raster}
      kind="intensity"
      axes={electrode.axes}
      maxHeight={420}
      quantity="|E| (V/m)"
      outline={outline(electrode)}
      lines={[
        { lines: outlineOfMode[0]?.lines ?? [], tone: "mode" },
        { lines: outlineOfMode[1]?.lines ?? [], tone: "mode-faint" },
      ]}
      scale={[lo, top]}
      ends={[`${short(lo)} V/m`, `${top < hi ? "≥ " : ""}${short(top)} V/m`]}
    />
  </section>
{/if}

{#each own as s, k (k)}
  {@const [lo, hi] = range(s.raster)}
  <section class="panel p-5">
    <h3 class="mb-1 font-semibold">{s.label[0].toUpperCase() + s.label.slice(1)} <span class="font-normal faint">· {s.unit}, on the cross-section</span></h3>
    <p class="mb-3 text-xs faint">
      from <span class="num">{fmt(lo)}</span> {s.unit} (black) to <span class="num">{fmt(hi)}</span> {s.unit} (pale yellow), {s.axes[0]} across and {s.axes[1]} up{#if heat(s) && step(s) > 0}; thin lines, isotherms every
        <span class="num">{tick(step(s))}</span> {s.unit}, on the solver's cells as the run recorded them (stepped where the cells are coarse){/if}
    </p>
    <RasterView
      raster={s.raster}
      kind="intensity"
      axes={s.axes}
      maxHeight={420}
      quantity="{s.label} ({s.unit})"
      outline={outline(s)}
      lines={heat(s) ? [{ lines: (isotherms[k] ?? []).flatMap((i) => i.lines), tone: "level" }] : []}
    />
  </section>
{/each}
