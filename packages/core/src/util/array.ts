export type LastResult<T> =
  | { readonly _tag: "some"; readonly value: T }
  | { readonly _tag: "none" }

export function findLast<T>(
  items: readonly T[],
  predicate: (item: T, index: number, items: readonly T[]) => boolean,
): LastResult<T> {
  for (let i = items.length - 1; i >= 0; i -= 1) {
    const item = items[i]
    if (predicate(item, i, items)) return { _tag: "some", value: item }
  }
  return { _tag: "none" }
}
