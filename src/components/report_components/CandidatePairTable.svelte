<script>
  import { getContext } from "svelte";
  import tooltip from "../../tooltip";

  export let data;
  export let rowLabel;
  export let colLabel;
export let generateTooltip;
  const { getCandidate } = getContext("candidates");

  function smooth(low, high, frac) {
    return low * (1 - frac) + high * frac;
  }

  // Add null safety for data.entries
  $: safeEntries = data?.entries || [];
  $: maxFrac = safeEntries.length > 0 
    ? Math.max(...safeEntries.map((row) => Math.max(...row.map((d) => (d ? d.frac : 0)))))
    : 1;

  function fracToColor(frac) {
    frac = frac / maxFrac;
    let h = smooth(0, 0, frac);
    let s = smooth(50, 95, frac);
    let l = smooth(97, 75, frac);

    return `hsl(${h} ${s}% ${l}%)`;
  }
</script>

<style>
  table {
    font-size: 8pt;
    margin: 0;
    cursor: pointer;
  }

  .colLabel div {
    transform: rotate(180deg);
    writing-mode: lr;
    margin: 0;
  }

  .colLabel {
    vertical-align: top;
  }

  .rowLabel {
    text-align: right;
  }

  .entry {
    height: 40px;
    width: 40px;
    font-size: 8pt;
    vertical-align: top;
    text-align: right;
    color: #333;
  }

  .colsLabel {
    text-align: right;
    font-size: 10pt;
    font-weight: bold;
    padding-bottom: 20px;
  }

  .rowsLabel {
    font-size: 10pt;
    font-weight: bold;
    padding-right: 20px;
  }

  .rowsLabel div {
    transform: rotate(180deg);
    writing-mode: lr;
  }
</style>

<table>
  <tbody>
    <tr>
      <td/>
      <td class="colsLabel" colspan={data.cols.length + 1}>{colLabel}</td>
    </tr>
    <tr>
      <td class="rowsLabel" rowspan={data.rows.length + 1}><div>{rowLabel}</div></td>
      <td />
      {#each data.cols as col}
        <td class="colLabel">
          <div>{getCandidate(col).name}</div>
        </td>
      {/each}
    </tr>
    {#each data.rows as row, i}
      <tr>
        <td class="rowLabel">{getCandidate(row).name}</td>
        {#each data.entries[i] as entry, j}
          <td
            use:tooltip={(generateTooltip && entry) ? generateTooltip(row, data.cols[j], entry) : ""}
            class="entry"
            style={entry ? `background: ${fracToColor(entry.frac)}` : ""}>
            {#if entry}{Math.round(entry.frac * 1000) / 10}%{/if}
          </td>
        {/each}
      </tr>
    {/each}
  </tbody>
</table>
