<script>
  import VoteCounts from "./report_components/VoteCounts.svelte";
  import Sankey from "./report_components/Sankey.svelte";
  // import CandidatePairTable from "./report_components/CandidatePairTable.svelte";
  import { EXHAUSTED } from "./candidates";

  import { onMount, setContext } from "svelte";

  export let report;

  // Add null safety for report data
  $: candidates = report?.candidates || {};
  $: reportInfo = report?.info || {};
  $: rounds = report?.rounds || [];
  $: candidatesArray = report?.candidates || [];
  $: numCandidates = report?.numCandidates || 0;
  $: winner = report?.winner || '';
  $: condorcet = report?.condorcet || '';
  $: ballotCount = report?.ballotCount || 0;

  function getCandidate(cid) {
    if (cid == "X") {
      return { name: "Exhausted", writeIn: false };
    } else {
      return candidates[cid] || { name: "Unknown", writeIn: false };
    }
  }

  setContext("candidates", {
    getCandidate,
  });

  function formatDate(dateStr) {
    let date = new Date(dateStr);
    const months = [
      "January",
      "February",
      "March",
      "April",
      "May",
      "June",
      "July",
      "August",
      "September",
      "October",
      "November",
      "Decemner",
    ];

    return `${
      months[date.getUTCMonth()]
    } ${date.getUTCDate()}, ${date.getUTCFullYear()}`;
  }
</script>

<div class="row">
  <p class="description" />
  <div class="electionHeader">
    <h3>
      <a href="/">rcv.report</a>
      //
      <strong>{report.info.jurisdictionName}</strong>
      {report.info.officeName}
    </h3>
  </div>
</div>

<div class="row">
  <div class="leftCol">
    <p>
      The
      {#if report.info.website}
      <a href={report.info.website}>{report.info.jurisdictionName} {report.info.electionName}</a>
      {:else}
      {report.info.jurisdictionName} {report.info.electionName}
      {/if}
      was held on
      <strong>{formatDate(report.info.date)}</strong>.
      <strong>{getCandidate(winner).name}</strong>
      was the winner out of
      <strong>{numCandidates}</strong>&nbsp;{#if numCandidates == 1}candidate {:else}candidates {/if}{#if rounds.length > 1}after
        {" "}<strong>{rounds.length - 1}</strong>&nbsp;elimination {#if rounds.length == 2}round{:else}rounds{/if}.
      {:else}. No elimination rounds were necessary to determine the outcome.
      {/if}
    </p>
    <p>
      {#if winner && condorcet}
        {#if winner == condorcet}
          <strong>{getCandidate(winner).name}</strong> was also the <a href="https://en.wikipedia.org/wiki/Condorcet_method">Condorcet winner</a>.
        {:else}
          <strong>{getCandidate(condorcet).name}</strong> was the <a href="https://en.wikipedia.org/wiki/Condorcet_method">Condorcet winner</a>.
        {/if}
      {/if}
    </p>
  </div>
  <div class="rightCol">
    <VoteCounts candidateVotes={report.totalVotes || []} />
  </div>
</div>

{#if rounds.length > 1}
  <div class="row">
    <div class="leftCol">
      <h2>Runoff Rounds</h2>

      <p>
        This <a href="https://en.wikipedia.org/wiki/Sankey_diagram">Sankey diagram</a> shows the votes of each remaining candidate at each round,
        as well as the breakdown of votes transferred when each candidate was
        eliminated.
      </p>

      <p>
        Note that the tabulation (but not the winner) may differ from the official count. You
        can <a href="/discrepancies">read more about why this is</a>.
      </p>
    </div>

    <div class="rightCol">
      <Sankey rounds={rounds} />
    </div>
  </div>
{/if}

{#if false && numCandidates > 1 && report.pairwisePreferences}
<div class="row">
  <div class="leftCol">
    <h2>Pairwise Preferences</h2>
    <p>
      For every pair of candidates, this table shows the fraction of voters who
      preferred one to the other. A preference means that either a voter ranks a
      candidate ahead of the other, or ranks one candidate but does not list the
      other. Ballots which rank neither candidate are not counted towards the
      percent counts.
    </p>
  </div>

  <div class="rightCol">
    {#if false && report.pairwisePreferences && report.pairwisePreferences.entries}
      <CandidatePairTable
        data={report.pairwisePreferences}
        rowLabel="Preferred Candidate"
        colLabel="Less-preferred Candidate"
        generateTooltip={(row, col, entry) => `
          Of the <strong>${entry.denominator.toLocaleString()}</strong> voters
          who expressed a preference, <strong>${Math.round(entry.frac * 1000) / 10}%</strong>
          (<strong>${entry.numerator.toLocaleString()}</strong>) preferred
          <strong>${getCandidate(row).name}</strong> to <strong>${getCandidate(col).name}</strong>.
        `} />
    {:else}
      <p><em>Pairwise preference analysis not available for this election.</em></p>
    {/if}
  </div>
</div>

<div class="row">
  <div class="leftCol">
    <h2>First Alternate</h2>
    <p>
      For every pair of candidates, this table shows the fraction of voters who
      ranked one candidate first ranked the other candidate second.
    </p>
  </div>

  <div class="rightCol">
    {#if false && report.firstAlternate && report.firstAlternate.entries}
      <CandidatePairTable
        generateTooltip={(row, col, entry) => (col !== EXHAUSTED ? `
          Of the <strong>${entry.denominator.toLocaleString()}</strong> voters who chose <strong>${getCandidate(row).name}</strong>
          as their first choice, <strong>${entry.numerator.toLocaleString()}</strong>
          (<strong>${Math.round(entry.frac * 1000) / 10}%</strong>)
          chose <strong>${getCandidate(col).name}</strong>
          as their second choice.
          ` : `
          Of the <strong>${entry.denominator.toLocaleString()}</strong> voters who chose <strong>${getCandidate(row).name}</strong>
          as their first choice, <strong>${entry.numerator.toLocaleString()}</strong>
          (<strong>${Math.round(entry.frac * 1000) / 10}%</strong>)
          did not rank another candidate.
         `)}
        data={report.firstAlternate}
        rowLabel="First Choice"
        colLabel="Second Choice" />
    {:else}
      <p><em>First alternate analysis not available for this election.</em></p>
    {/if}
  </div>
</div>
{/if}

{#if rounds.length > 1}
  <div class="row">
    <div class="leftCol">
      <h2>Final Vote by First Choice</h2>
      <p>
        This table tracks which candidate ballots were ultimately allocated to,
        among ballots that ranked an eliminated candidate first.
      </p>
    </div>

    <div class="rightCol">
      {#if false && report.firstFinal && report.firstFinal.entries}
        <CandidatePairTable
          generateTooltip={(row, col, entry) => (col !== EXHAUSTED ? `
          Of the <strong>${entry.denominator.toLocaleString()}</strong> ballots that ranked <strong>${getCandidate(row).name}</strong>
          first, <strong>${entry.numerator.toLocaleString()}</strong>
          (<strong>${Math.round(entry.frac * 1000) / 10}%</strong>)
          were allocated to <strong>${getCandidate(col).name}</strong>
          in the final round.
          ` : `
          Of the <strong>${entry.denominator.toLocaleString()}</strong> ballots that ranked <strong>${getCandidate(row).name}</strong>
          first, <strong>${entry.numerator.toLocaleString()}</strong>
          (<strong>${Math.round(entry.frac * 1000) / 10}%</strong>)
          were exhausted by the final round.
          `)}
          data={report.firstFinal}
          rowLabel="First Round Choice"
          colLabel="Final Round Choice" />
      {:else}
        <p><em>First-final ranking analysis not available for this election.</em></p>
      {/if}
    </div>
  </div>
{/if}
