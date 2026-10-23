// Keeps every block's ID present and unique, also after splitting a block
// (Enter), pasting or merging changes from another device.

import { type Node as PMNode } from "prosemirror-model";
import { Plugin, type Transaction } from "prosemirror-state";
import { BLOCKS_WITH_ID } from "./schema";

const ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/** A random 10-character block ID (about 59 bits). */
export function newBid(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(10));
  return Array.from(bytes, (b) => ALPHABET[b % ALPHABET.length]).join("");
}

/** Returns a transaction that fixes missing or repeated block IDs, or null. */
export function fixBids(doc: PMNode, tr: Transaction): Transaction | null {
  const seen = new Set<string>();
  let changed = false;
  doc.descendants((node, pos) => {
    if (!BLOCKS_WITH_ID.has(node.type.name)) return true;
    const bid = node.attrs.bid as string | null;
    if (!bid || seen.has(bid)) {
      const fresh = newBid();
      tr.setNodeAttribute(pos, "bid", fresh);
      seen.add(fresh);
      changed = true;
    } else {
      seen.add(bid);
    }
    return true;
  });
  return changed ? tr : null;
}

export const bidsPlugin = new Plugin({
  appendTransaction(transactions, _old, state) {
    if (!transactions.some((tr) => tr.docChanged)) return null;
    return fixBids(state.doc, state.tr);
  },
});
