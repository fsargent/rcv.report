<script>
  import { getContext } from "svelte";
  import tooltip from "../../tooltip";

  export let candidateVotes;

  const { getCandidate } = getContext("candidates");

  const outerHeight = 24;
  const innerHeight = 14;
  const labelSpace = 130;
  const width = 600;

  // Add null safety and handle different data structures
  $: safeVotes = candidateVotes || [];
  $: maxVotes = safeVotes.length > 0 
    ? Math.max(...safeVotes.map((d) => (d.firstRoundVotes || 0) + (d.transferVotes || 0) + (d.votes || 0)))
    : 1;
  $: scale = (width - labelSpace - 15) / maxVotes;
  $: height = outerHeight * safeVotes.length;
</script>

<style>
  .firstRound {
    fill: #aa0d0d;
  }

  .transfer {
    fill: #e7ada0;
  }

  .eliminated {
    opacity: 30%;
  }
</style>

<svg width="100%" viewBox={`0 0 ${width} ${height}`}>
  <g transform={`translate(${labelSpace} 0)`}>
    {#each safeVotes as votes, i}
      <g
        class={votes.roundEliminated === null ? '' : 'eliminated'}
        transform={`translate(0 ${outerHeight * (i + 0.5)})`}>
        <text font-size="12" text-anchor="end" dominant-baseline="middle">
          {getCandidate(votes.candidate).name}
        </text>
        <g transform={`translate(5 ${-innerHeight / 2 - 1})`}>
          <rect
            class="firstRound"
            height={innerHeight}
            width={scale * (votes.firstRoundVotes || votes.votes || 0)}
            use:tooltip={`<strong>${getCandidate(votes.candidate || votes.name).name || votes.name}</strong>
            received <strong>${(votes.firstRoundVotes || votes.votes || 0).toLocaleString()}</strong> votes
            in the first round.`} />
          <rect
            class="transfer"
            x={scale * (votes.firstRoundVotes || votes.votes || 0)}
            height={innerHeight}
            width={scale * (votes.transferVotes || 0)}
            use:tooltip={`<strong>${getCandidate(votes.candidate || votes.name).name || votes.name}</strong>
            received <strong>${(votes.transferVotes || 0).toLocaleString()}</strong> transfer votes.`}
            />
        </g>
        {#if votes.roundEliminated !== null}
            <text
            font-size="12"
            dominant-baseline="middle"
            x={10 + scale * (votes.firstRoundVotes + votes.transferVotes)}>
            Eliminated in round {votes.roundEliminated}
            </text>
        {/if}
      </g>
    {/each}
  </g>
</svg>
