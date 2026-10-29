# ALTAIS – texty rozhraní (čeština). Formát: Project Fluent.

appName = ALTAIS
loading = Načítá se…
welcomeTitle = Vítej v ALTAIS
welcomeText = Poznámky budou uloženy v šifrovaném archivu na tomto počítači. Klíč k němu si bezpečně zapamatuje Windows, takže heslo není potřeba. Heslo si můžeš zapnout i teď.
archiveFolder = Složka archivu
usePassword = Chránit archiv i heslem
passwordRequired = Tento systém si klíč zapamatovat neumí, proto je heslo povinné.
password = Heslo
passwordAgain = Heslo ještě jednou
passwordsDiffer = Hesla se neshodují.
createArchive = Vytvořit archiv
recoveryTitle = Tvůj záchranný klíč
recoveryText = Tímto klíčem otevřeš archiv na jiném počítači nebo když zapomeneš heslo. Zapiš si ho nebo vytiskni a ulož na bezpečné místo. Zobrazí se jen teď.
copy = Kopírovat
copied = Zkopírováno
recoverySaved = Záchranný klíč mám bezpečně uložený
continue = Pokračovat
unlockTitle = Otevřít archiv
unlockWithPassword = Heslem
unlockWithRecovery = Záchranným klíčem
recoveryKey = Záchranný klíč
open = Otevřít
allNotes = Všechny poznámky
notebooks = Sešity
newNotebook = Nový sešit
notebookName = Název sešitu
create = Vytvořit
cancel = Zrušit
importEvernote = Import z Evernote
importing = Importuje se…
importDone = Import dokončen
importSummary = { $notes ->
        [one] { $notes } poznámka
        [few] { $notes } poznámky
       *[other] { $notes } poznámek
    } a { $attachments ->
        [one] { $attachments } příloha
        [few] { $attachments } přílohy
       *[other] { $attachments } příloh
    } do sešitu „{ $notebook }“
partlyImported = Částečně importováno:
stoppedEarly = Soubor se nepodařilo dočíst do konce:
close = Zavřít
search = Hledat…
noNotes = Žádné poznámky
noResults = Nic nenalezeno
newNote = Nová poznámka
untitled = Bez názvu
titlePlaceholder = Název
saving = Ukládá se…
saved = Uloženo
saveFailed = Uložení selhalo
trash = Do koše
selectNote = Vyber poznámku nebo vytvoř novou.
settings = Nastavení
passwordOn = Archiv je chráněn heslem. Při otevření si ho ALTAIS vyžádá.
passwordOff = Heslo není nastaveno. Klíč k archivu bezpečně uchovává Windows.
setPassword = Nastavit heslo
changePassword = Změnit heslo
removePassword = Zrušit heslo
newPassword = Nové heslo
savePassword = Uložit heslo
passwordSet = Heslo je nastaveno.
passwordRemoved = Heslo je zrušeno. Klíč opět uchovává Windows.
recoveryKeyHint = Pokud jsi starý záchranný klíč ztratil, vytvoř nový. Starý tím přestane fungovat.
createNewRecoveryKey = Vytvořit nový záchranný klíč
newRecoveryKeyTitle = Nový záchranný klíč
newRecoveryKeyText = Starý záchranný klíč už nefunguje. Zapiš si nový a ulož ho na bezpečné místo. Zobrazí se jen teď.
recoveryReplaced = Záchranný klíč byl vyměněn.
or = nebo
tryDemo = Vyzkoušet s ukázkovými daty
tryDemoHint = Ukázka se uloží odděleně od tvých poznámek a dá se kdykoli úplně odstranit.
demoTitle = Ukázková data
demoText = Zkoušíš ALTAIS na ukázkových poznámkách. Až budeš připraven, ukázku odstraň a začni s vlastními.
removeDemo = Odstranit ukázku a začít
removeDemoTitle = Odstranit ukázku?
removeDemoText = Ukázkové poznámky i jejich soubory se úplně smažou. Z ukázky v programu nic nezůstane a tvé vlastní poznámky to neovlivní.
removeDemoConfirm = Odstranit ukázku
lock = Zamknout
saveAs = Uložit jako…
openAttachment = Otevřít
addAttachment = Přidat přílohu
cannotLoadAttachment = Přílohu se nepodařilo načíst.
language = Jazyk
languageAuto = Podle systému
languageHint = Změna se projeví po novém spuštění ALTAIS.

## tools

tool-bold = Tučné (Ctrl+B)
tool-italic = Kurzíva (Ctrl+I)
tool-underline = Podtržené (Ctrl+U)
tool-strike = Přeškrtnuté
tool-code = Kód
tool-paragraph = Odstavec
tool-heading1 = Nadpis 1
tool-heading2 = Nadpis 2
tool-heading3 = Nadpis 3
tool-bulletList = Odrážky
tool-orderedList = Číslovaný seznam
tool-checkList = Seznam úkolů
tool-undo = Zpět (Ctrl+Z)
tool-redo = Znovu (Ctrl+Y)

## errors

error-wrong-password = Nesprávné heslo.
error-wrong-recovery-key = Tento záchranný klíč nepatří k tomuto archivu.
error-bad-recovery-key = Toto není platný záchranný klíč. Zkontroluj překlepy.
error-key-not-on-this-computer = Klíč k archivu není uložen na tomto počítači. Použij heslo nebo záchranný klíč.
error-cannot-decrypt = Archiv nelze dešifrovat: soubor je poškozený nebo nepatří k souboru klíčů.
error-newer-version = Archiv vytvořila novější verze ALTAIS. Aktualizuj program.
error-empty-name = Zadej název.
error-demo-only = V ukázce to není dostupné.
error-damaged-demo-package = Balík ukázkových dat je poškozený.
error-missing-demo-package = Balík ukázkových dat chybí.
error-cannot-remove-demo = Ukázku se nepodařilo odstranit.
error-empty-password = Heslo nesmí být prázdné.
error-no-protection = Bez hesla může archiv chránit jen Windows; na tomto systému to nejde.
error-cannot-read-file = Soubor nelze přečíst.
error-cannot-write-file = Soubor nelze uložit.
error-missing-attachment = Příloha v archivu chybí.
error-damaged-attachment = Soubor přílohy chybí nebo je poškozený.
error-unsafe-to-open = Tento soubor by se spustil jako program, proto ho ALTAIS neotevře. Můžeš ho uložit.
error-cannot-open-file = Soubor se nepodařilo otevřít.
error-invalid-note = Poznámka obsahuje neplatné údaje.
error-unknown = Něco se pokazilo.
