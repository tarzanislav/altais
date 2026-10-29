# ALTAIS – interface texts (English). Format: Project Fluent.

appName = ALTAIS
loading = Loading…
welcomeTitle = Welcome to ALTAIS
welcomeText = Your notes are kept in an encrypted archive on this computer. Windows keeps its key safely, so no password is needed. You can add a password now if you like.
archiveFolder = Archive folder
usePassword = Also protect the archive with a password
passwordRequired = This system cannot keep the key, so a password is required.
password = Password
passwordAgain = Password again
passwordsDiffer = The passwords differ.
createArchive = Create archive
recoveryTitle = Your recovery key
recoveryText = This key opens the archive on another computer or if you forget the password. Write it down or print it and keep it safe. It is shown only now.
copy = Copy
copied = Copied
recoverySaved = I have saved the recovery key
continue = Continue
unlockTitle = Open the archive
unlockWithPassword = Password
unlockWithRecovery = Recovery key
recoveryKey = Recovery key
open = Open
allNotes = All notes
notebooks = Notebooks
newNotebook = New notebook
notebookName = Notebook name
create = Create
cancel = Cancel
importEvernote = Import from Evernote
importing = Importing…
importDone = Import finished
importSummary = { $notes ->
        [one] { $notes } note
       *[other] { $notes } notes
    } and { $attachments ->
        [one] { $attachments } attachment
       *[other] { $attachments } attachments
    } into “{ $notebook }”
partlyImported = Partly imported:
stoppedEarly = The file could not be read to the end:
close = Close
search = Search…
noNotes = No notes
noResults = Nothing found
newNote = New note
untitled = Untitled
titlePlaceholder = Title
saving = Saving…
saved = Saved
saveFailed = Saving failed
trash = Move to trash
selectNote = Select a note or create a new one.
settings = Settings
passwordOn = The archive is protected by a password. ALTAIS asks for it when opening.
passwordOff = No password is set. Windows keeps the archive key safe.
setPassword = Set a password
changePassword = Change password
removePassword = Remove password
newPassword = New password
savePassword = Save password
passwordSet = The password is set.
passwordRemoved = The password is removed. Windows keeps the key again.
recoveryKeyHint = If you lost the old recovery key, create a new one. The old one then stops working.
createNewRecoveryKey = Create a new recovery key
newRecoveryKeyTitle = New recovery key
newRecoveryKeyText = The old recovery key no longer works. Write down the new one and keep it safe. It is shown only now.
recoveryReplaced = The recovery key was replaced.
or = or
tryDemo = Try ALTAIS with demo data
tryDemoHint = The demo is kept apart from your notes and can be removed completely at any time.
demoTitle = Demo data
demoText = You are trying ALTAIS with demo notes. When you are ready, remove the demo and start with your own.
removeDemo = Remove demo and start
removeDemoTitle = Remove the demo?
removeDemoText = The demo notes and their files are deleted completely. Nothing of the demo stays in the app, and your own notes are not affected.
removeDemoConfirm = Remove demo
lock = Lock
saveAs = Save as…
openAttachment = Open
addAttachment = Add attachment
cannotLoadAttachment = The attachment could not be loaded.
language = Language
languageAuto = As in the system
languageHint = The change applies when ALTAIS starts again.
trashTitle = Trash
trashEmpty = The trash is empty
emptyTrash = Empty trash
emptyTrashTitle = Empty the trash?
emptyTrashText = { $count ->
        [one] { $count } note is deleted for good, with attachments no other note uses. This cannot be undone.
       *[other] { $count } notes are deleted for good, with attachments no other note uses. This cannot be undone.
    }
inTrash = This note is in the trash.
restore = Restore
deleteForever = Delete for good
deleteForeverTitle = Delete the note for good?
deleteForeverText = The note is deleted with attachments no other note uses. This cannot be undone.

## tools

tool-bold = Bold (Ctrl+B)
tool-italic = Italic (Ctrl+I)
tool-underline = Underline (Ctrl+U)
tool-strike = Strikethrough
tool-code = Code
tool-paragraph = Paragraph
tool-heading1 = Heading 1
tool-heading2 = Heading 2
tool-heading3 = Heading 3
tool-bulletList = Bulleted list
tool-orderedList = Numbered list
tool-checkList = Checklist
tool-undo = Undo (Ctrl+Z)
tool-redo = Redo (Ctrl+Y)

## errors

error-wrong-password = Wrong password.
error-wrong-recovery-key = This recovery key does not belong to this archive.
error-bad-recovery-key = This is not a valid recovery key. Check for typos.
error-key-not-on-this-computer = The archive key is not stored on this computer. Use the password or the recovery key.
error-cannot-decrypt = The archive cannot be decrypted: the file is damaged or does not match the key file.
error-newer-version = A newer version of ALTAIS created this archive. Please update.
error-empty-name = Enter a name.
error-demo-only = Not available in the demo.
error-damaged-demo-package = The demo package is damaged.
error-missing-demo-package = The demo package is missing.
error-cannot-remove-demo = The demo could not be removed.
error-empty-password = The password must not be empty.
error-no-protection = Without a password only Windows can protect the archive; not possible on this system.
error-cannot-read-file = The file cannot be read.
error-cannot-write-file = The file cannot be written.
error-missing-attachment = The attachment is missing from the archive.
error-damaged-attachment = The attachment's file is missing or damaged.
error-unsafe-to-open = This file would run as a program, so ALTAIS does not open it. You can save it.
error-cannot-open-file = The file could not be opened.
error-invalid-note = The note contains invalid data.
error-not-in-trash = Only notes in the trash can be deleted for good.
error-unknown = Something went wrong.

