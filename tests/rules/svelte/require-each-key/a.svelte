{#each items as item}
^^^^^^^^^^^^^^^^^^^^^ require-each-key
  <p>{item}</p>
{/each}
{#each items as item (item.id)}
  <p>{item}</p>
{/each}
{#each items as { id, name }, i (id)}{name}{/each}
