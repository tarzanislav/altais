# ALTAIS – texty rozhrania (slovenčina, zdroj pre ostatné jazyky).
# Formát: Project Fluent, https://projectfluent.org
# Každý jazyk má rovnaké správy; chýbajúca správa sa zobrazí po anglicky.

appName = ALTAIS
loading = Načítava sa…
welcomeTitle = Vitaj v ALTAIS
welcomeText = Poznámky budú uložené v šifrovanom archíve na tomto počítači. Kľúč k nemu si bezpečne zapamätá Windows, takže heslo nie je potrebné. Heslo si môžeš zapnúť aj teraz.
archiveFolder = Priečinok archívu
usePassword = Chrániť archív aj heslom
passwordRequired = Tento systém si kľúč zapamätať nevie, preto je heslo povinné.
password = Heslo
passwordAgain = Heslo ešte raz
passwordsDiffer = Heslá sa nezhodujú.
createArchive = Vytvoriť archív
recoveryTitle = Tvoj záchranný kľúč
recoveryText = Týmto kľúčom otvoríš archív na inom počítači alebo keď zabudneš heslo. Zapíš si ho alebo vytlač a ulož na bezpečné miesto. Ukáže sa iba teraz.
copy = Kopírovať
copied = Skopírované
recoverySaved = Záchranný kľúč mám bezpečne uložený
continue = Pokračovať
unlockTitle = Otvoriť archív
unlockWithPassword = Heslom
unlockWithRecovery = Záchranným kľúčom
recoveryKey = Záchranný kľúč
open = Otvoriť
allNotes = Všetky poznámky
notebooks = Zošity
newNotebook = Nový zošit
notebookName = Názov zošita
create = Vytvoriť
cancel = Zrušiť
importEvernote = Import z Evernote
importing = Importuje sa…
importDone = Import dokončený
importSummary = { $notes ->
        [one] { $notes } poznámka
        [few] { $notes } poznámky
       *[other] { $notes } poznámok
    } a { $attachments ->
        [one] { $attachments } príloha
        [few] { $attachments } prílohy
       *[other] { $attachments } príloh
    } do zošita „{ $notebook }“
partlyImported = Čiastočne importované:
stoppedEarly = Súbor sa nedal dočítať do konca:
close = Zavrieť
search = Hľadať…
noNotes = Žiadne poznámky
noResults = Nič sa nenašlo
newNote = Nová poznámka
untitled = Bez názvu
titlePlaceholder = Názov
saving = Ukladá sa…
saved = Uložené
saveFailed = Uloženie zlyhalo
trash = Do koša
selectNote = Vyber poznámku alebo vytvor novú.
settings = Nastavenia
passwordOn = Archív je chránený heslom. Pri otvorení si ho ALTAIS vypýta.
passwordOff = Heslo nie je nastavené. Kľúč k archívu bezpečne uchováva Windows.
setPassword = Nastaviť heslo
changePassword = Zmeniť heslo
removePassword = Zrušiť heslo
newPassword = Nové heslo
savePassword = Uložiť heslo
passwordSet = Heslo je nastavené.
passwordRemoved = Heslo je zrušené. Kľúč opäť uchováva Windows.
recoveryKeyHint = Ak si starý záchranný kľúč stratil, vytvor nový. Starý tým prestane fungovať.
createNewRecoveryKey = Vytvoriť nový záchranný kľúč
newRecoveryKeyTitle = Nový záchranný kľúč
newRecoveryKeyText = Starý záchranný kľúč už nefunguje. Zapíš si nový a ulož ho na bezpečné miesto. Ukáže sa iba teraz.
recoveryReplaced = Záchranný kľúč bol vymenený.
or = alebo
tryDemo = Vyskúšať s ukážkovými dátami
tryDemoHint = Ukážka sa uloží oddelene od tvojich poznámok a dá sa kedykoľvek úplne odstrániť.
demoTitle = Ukážkové dáta
demoText = Skúšaš ALTAIS na ukážkových poznámkach. Keď budeš pripravený, ukážku odstráň a začni s vlastnými.
removeDemo = Odstrániť ukážku a začať
removeDemoTitle = Odstrániť ukážku?
removeDemoText = Ukážkové poznámky aj ich súbory sa úplne zmažú. Z ukážky v programe nič neostane a tvoje vlastné poznámky to neovplyvní.
removeDemoConfirm = Odstrániť ukážku
lock = Zamknúť
saveAs = Uložiť ako…
openAttachment = Otvoriť
addAttachment = Pridať prílohu
cannotLoadAttachment = Prílohu sa nepodarilo načítať.
language = Jazyk
languageAuto = Podľa systému
languageHint = Zmena sa prejaví po opätovnom spustení ALTAIS.

## tools

tool-bold = Tučné (Ctrl+B)
tool-italic = Kurzíva (Ctrl+I)
tool-underline = Podčiarknuté (Ctrl+U)
tool-strike = Prečiarknuté
tool-code = Kód
tool-paragraph = Odsek
tool-heading1 = Nadpis 1
tool-heading2 = Nadpis 2
tool-heading3 = Nadpis 3
tool-bulletList = Odrážky
tool-orderedList = Číslovaný zoznam
tool-checkList = Zoznam úloh
tool-undo = Späť (Ctrl+Z)
tool-redo = Znova (Ctrl+Y)

## errors

error-wrong-password = Nesprávne heslo.
error-wrong-recovery-key = Tento záchranný kľúč nepatrí k tomuto archívu.
error-bad-recovery-key = Toto nie je platný záchranný kľúč. Skontroluj preklepy.
error-key-not-on-this-computer = Kľúč k archívu nie je uložený na tomto počítači. Použi heslo alebo záchranný kľúč.
error-cannot-decrypt = Archív sa nedá dešifrovať: súbor je poškodený alebo nepatrí k súboru kľúčov.
error-newer-version = Archív vytvorila novšia verzia ALTAIS. Aktualizuj program.
error-empty-name = Zadaj názov.
error-demo-only = V ukážke to nie je dostupné.
error-damaged-demo-package = Balík ukážkových dát je poškodený.
error-missing-demo-package = Balík ukážkových dát chýba.
error-cannot-remove-demo = Ukážku sa nepodarilo odstrániť.
error-empty-password = Heslo nesmie byť prázdne.
error-no-protection = Bez hesla môže archív chrániť iba Windows; na tomto systéme to nejde.
error-cannot-read-file = Súbor sa nedá prečítať.
error-cannot-write-file = Súbor sa nedá uložiť.
error-missing-attachment = Príloha v archíve chýba.
error-damaged-attachment = Súbor prílohy chýba alebo je poškodený.
error-unsafe-to-open = Tento súbor by sa spustil ako program, preto ho ALTAIS neotvorí. Môžeš ho uložiť.
error-cannot-open-file = Súbor sa nepodarilo otvoriť.
error-invalid-note = Poznámka obsahuje neplatné údaje.
error-unknown = Niečo sa pokazilo.

