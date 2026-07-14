<script lang="ts">
  import type {
    ICandidateVotes,
    CandidateId,
    ICandidate,
  } from "../../report_types";
  import type { CandidateContext } from "../candidates";
  import { getContext } from "svelte";
  import tooltip from "../../tooltip";

  export let candidateVotes: ICandidateVotes[];
  export let winner: CandidateId | undefined = undefined;

  const { getCandidate } = getContext("candidates") as CandidateContext;

  const outerHeight = 24;
  const innerHeight = 14;
  const labelSpace = 130;
  const width = 600;

  // Sort by total votes (firstRoundVotes + transferVotes) in descending order
  $: sortedCandidateVotes = [...candidateVotes].sort((a, b) =>
    (b.firstRoundVotes + b.transferVotes) - (a.firstRoundVotes + a.transferVotes)
  );

  $: maxVotes = Math.max(...sortedCandidateVotes.map((d) => d.firstRoundVotes + d.transferVotes));
  $: scale = (width - labelSpace - 15) / maxVotes;

  $: height = outerHeight * sortedCandidateVotes.length;
</script>

<style>
  svg {
    color-scheme: light;
  }

  .firstRound {
    fill: #aa0d0d;
  }

  .transfer {
    fill: #e7ada0;
  }

  .winner .firstRound {
    fill: #700000;
  }

  .winner .transfer {
    fill: #c8786b;
  }

  .eliminated {
    opacity: 30%;
  }

  @media (prefers-color-scheme: dark) {
    /* Make all text in SVG light colored */
    text {
      fill: #e3e3e3;
    }
  }
</style>

<svg width="100%" viewBox={`0 0 ${width} ${height}`}>
  <g transform={`translate(${labelSpace} 0)`}>
    {#each sortedCandidateVotes as votes, i}
      <g
        class:winner={votes.candidate === winner}
        class:eliminated={votes.roundEliminated != null}
        transform={`translate(0 ${outerHeight * (i + 0.5)})`}>
        <text font-size="12" text-anchor="end" dominant-baseline="middle">
          {getCandidate(votes.candidate).name}
        </text>
        <g transform={`translate(5 ${-innerHeight / 2 - 1})`}>
          <rect
            class="firstRound"
            height={innerHeight}
            width={scale * votes.firstRoundVotes}
            use:tooltip={`<strong>${getCandidate(votes.candidate).name}</strong>
            received <strong>${votes.firstRoundVotes.toLocaleString()}</strong> votes
            in the first round.`} />
          <rect
            class="transfer"
            x={scale * votes.firstRoundVotes}
            height={innerHeight}
            width={scale * votes.transferVotes}
            use:tooltip={`<strong>${getCandidate(votes.candidate).name}</strong>
            received <strong>${votes.transferVotes.toLocaleString()}</strong> transfer votes.`}
            />
        </g>
        {#if votes.roundEliminated != null}
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
