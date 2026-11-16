<head></head>
 ^^^^ no-raw-special-elements
<svelte:head><title>t</title></svelte:head>
<input on:click={a} on:click={b} on:click|once={c} use:tip use:tip use:other style:color="a" style:color|important={b}>
//                  ^^^^^^^^ no-dupe-on-directives
//                                                         ^^^^^^^ no-dupe-use-directives
//                                                                                           ^^^^^^^^^^^^^^^^^^^^^ no-dupe-style-properties
<input type="checkbox" bind:value={v} bind:checked={v} id={id} {id} bind:group={group} class:on={on}>
//                     ^^^^^^^^^^ no-bind-value-on-checkable-inputs
//                                                     ^^ shorthand-attribute
//                                                                  ^^^^^^^^^^ shorthand-directive
//                                                                                     ^^^^^^^^ shorthand-directive
<input bind:value={value} class:big>
//     ^^^^^^^^^^ shorthand-directive
<C>{#snippet children()}hi{/snippet}</C>
// ^^^^^^^^^^^^^^^^^^^^^ no-useless-children-snippet
<C>{#snippet children(x)}{x}{/snippet}</C>
<p>{"text"} {'it'} {`tpl`} {`a${b}`} {"a'b"} {text}</p>
// ^^^^^^^^ no-useless-mustaches
//          ^^^^^^ no-useless-mustaches
//                 ^^^^^^^ no-useless-mustaches
//                                   ^^^^^^^ no-useless-mustaches
<!--bad--> <!-- ok --> <!---->
^^^^^^^^^^ spaced-html-comment
<div><style>a { color: red }</style></div>
//    ^^^^^ no-nested-style-tag
<a href="https://x.org" target="_blank">x</a>
//                      ^^^^^^ no-target-blank
<a href="https://x.org" target="_blank" rel="noopener noreferrer">x</a>
<a href="/local" target="_blank">x</a>
