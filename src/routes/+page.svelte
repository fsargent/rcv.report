<script>
  import ElectionIndex from "../components/ElectionIndex.svelte";

  export let data;
  $: ({ elections } = data);
</script>

<title>rcv.report: detailed reports on ranked-choice elections.</title>

<div class="wide container">
<div class="row">
  <div class="leftCol">
    <div class="description">
      <h1>rcv.report</h1>:
      detailed reports on ranked-choice elections.
    </div>
    <p>
      Ranked-choice elections produce more data on voter preferences than
      pick-one elections. <strong>rcv.report</strong> runs analysis on the
      ballot-level data and publishes reports on each election.
    </p>

    <p>
      rcv.report is a fork of ranked.vote which was created by
      <a href="https://paulbutler.org">Paul Butler</a>.
      rcv.report is maintained by
      <a href="https://github.com/fsargent">Felix Sargent</a>.
      It is non-partisan and has received no outside funding.
    </p>
  </div>

  <div class="rightCol">
    <ElectionIndex {elections} />
  </div>
</div>
</div>