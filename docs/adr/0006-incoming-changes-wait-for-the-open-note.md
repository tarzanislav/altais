# ADR-0006: Changes from other devices wait for the open note

- Status: accepted
- Date: 2026-10-22

## Context

Loro merges changes from other devices automatically. Applied to the note
the user is writing in, they make text move under the cursor, and the editor
spike showed that the ProseMirror binding (loro-prosemirror 0.4) does not keep
the cursor reliably: it jumped to the start of the note, and a click right
after a remote change could be undone by a delayed cursor restore.

## Decision

Changes for the note that is **open in the editor** do not change it while
the user writes:

1. Received changes are verified, decrypted and stored in a local **inbox**
   (in the encrypted database, same transaction as the sync cursor). Nothing
   is lost if the program stops before they are applied.
2. A bar above the note shows how many changes are waiting and offers
   **Update now**.
3. Waiting changes are applied:
   - when the user clicks **Update now**;
   - after a **pause in typing** in the program (default 30 s, adjustable in
     Settings), never while an input method is composing text;
   - when the program **closes**;
   - when the program **opens** or the note is opened again, shown as
     "Synchronizing…".
4. Applying keeps the cursor where it was (positions are mapped through the
   change) and ignores input for the few milliseconds the editor needs, so a
   click cannot be undone.

Everything else is unchanged: notes that are not open take changes
immediately; the user's own changes are sent immediately; merging is still
done by the CRDT, so changes made while others wait never conflict.

## Consequences

- No text moves under the cursor; the user decides or the pause decides.
- Two devices can show different versions of an open note for a while;
  the bar makes that visible.
- Search, lists and other notes always show the latest state, because only
  the open note waits.
- The spike (`apps/editor-spike`) implements and tests this behaviour.
