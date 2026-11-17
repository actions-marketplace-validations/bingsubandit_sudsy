<script>
  let { url, value, checked } = $props();
  function a() {}
  function b() {}
</script>

<head></head>
<svelte:head></svelte:head>

<button type="button" on:click={a} on:click={b}>twice</button>
<button type="button" on:click|once={a} on:click={b}>modifiers differ</button>
<button type="button" on:keydown={a} on:keydown|once={a}>same handler</button>
<p use:a use:a></p>
<p style:color="red" style:background="blue"></p>

<a href="https://example.org" target="_blank">no rel</a>
<a href="https://example.org" target="_blank" rel="noreferrer">only noreferrer</a>
<a href="https://example.org" target="_blank" rel="noopener noreferrer">both</a>
<a href={url} target="_blank">dynamic</a>
<a href="/local" target="_blank">local</a>

<input type="checkbox" bind:value />
<input type="checkbox" bind:checked />
<input type="text" bind:value />
<button>no type</button>
<button type="button">typed</button>
<p>{@debug value}</p>
<!--no space-->
<p>{'text'}</p>
{#each [1, 2] as item}
  <p id={item}>{item}</p>
{/each}
<input id={value} />
<input bind:value={value} />
