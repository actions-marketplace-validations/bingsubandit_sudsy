<head></head>
<svelte:head><title>t</title></svelte:head>
<input on:click={a} on:click={a} on:click|once={c} use:tip use:tip use:other style:color="a" style:color|important={b}>
<input type="checkbox" bind:value={v} bind:checked={v} {id} {id} bind:group class:on>
<input bind:value class:big>
<C>{#snippet children()}hi{/snippet}</C>
<C>{#snippet children(x)}{x}{/snippet}</C>
<p style:marginLeft="1px" style:margin-left="1px" style:--x="1"></p>
<p>{"text"} {'it'} {`tpl`} {`a${b}`} {"a'b"} {text}</p>
<p>{{ a: 1 }} {[1]} {function () {}} {text}</p>
<!--bad--> <!-- ok --> <!---->
<div><style>a { color: red }</style></div>
<a href="https://x.org" target="_blank">x</a>
<a href="https://x.org" target="_blank" rel="noopener noreferrer">x</a>
<a href="https://x.org" target="_blank" rel="noreferrer">x</a>
<a href="/local" target="_blank">x</a>
