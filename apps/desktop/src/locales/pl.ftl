# ALTAIS – teksty interfejsu (polski). Format: Project Fluent.

appName = ALTAIS
loading = Wczytywanie…
welcomeTitle = Witaj w ALTAIS
welcomeText = Notatki będą przechowywane w zaszyfrowanym archiwum na tym komputerze. Klucz do niego bezpiecznie zapamięta Windows, więc hasło nie jest potrzebne. Możesz też włączyć hasło już teraz.
archiveFolder = Folder archiwum
usePassword = Chroń archiwum także hasłem
passwordRequired = Ten system nie potrafi zapamiętać klucza, dlatego hasło jest wymagane.
password = Hasło
passwordAgain = Powtórz hasło
passwordsDiffer = Hasła się różnią.
createArchive = Utwórz archiwum
recoveryTitle = Twój klucz odzyskiwania
recoveryText = Tym kluczem otworzysz archiwum na innym komputerze albo gdy zapomnisz hasła. Zapisz go lub wydrukuj i przechowuj w bezpiecznym miejscu. Jest wyświetlany tylko teraz.
copy = Kopiuj
copied = Skopiowano
recoverySaved = Mam bezpiecznie zapisany klucz odzyskiwania
continue = Dalej
unlockTitle = Otwórz archiwum
unlockWithPassword = Hasłem
unlockWithRecovery = Kluczem odzyskiwania
recoveryKey = Klucz odzyskiwania
open = Otwórz
allNotes = Wszystkie notatki
notebooks = Notesy
newNotebook = Nowy notes
notebookName = Nazwa notesu
create = Utwórz
cancel = Anuluj
importEvernote = Import z Evernote
importing = Importowanie…
importDone = Import zakończony
importSummary = { $notes ->
        [one] { $notes } notatka
        [few] { $notes } notatki
       *[other] { $notes } notatek
    } i { $attachments ->
        [one] { $attachments } załącznik
        [few] { $attachments } załączniki
       *[other] { $attachments } załączników
    } do notesu „{ $notebook }”
partlyImported = Zaimportowano częściowo:
stoppedEarly = Nie udało się odczytać pliku do końca:
close = Zamknij
search = Szukaj…
noNotes = Brak notatek
noResults = Nic nie znaleziono
newNote = Nowa notatka
untitled = Bez tytułu
titlePlaceholder = Tytuł
saving = Zapisywanie…
saved = Zapisano
saveFailed = Zapis nie powiódł się
trash = Do kosza
selectNote = Wybierz notatkę lub utwórz nową.
settings = Ustawienia
passwordOn = Archiwum jest chronione hasłem. ALTAIS poprosi o nie przy otwieraniu.
passwordOff = Hasło nie jest ustawione. Klucz do archiwum bezpiecznie przechowuje Windows.
setPassword = Ustaw hasło
changePassword = Zmień hasło
removePassword = Usuń hasło
newPassword = Nowe hasło
savePassword = Zapisz hasło
passwordSet = Hasło jest ustawione.
passwordRemoved = Hasło zostało usunięte. Klucz znów przechowuje Windows.
recoveryKeyHint = Jeśli zgubiłeś stary klucz odzyskiwania, utwórz nowy. Stary przestanie wtedy działać.
createNewRecoveryKey = Utwórz nowy klucz odzyskiwania
newRecoveryKeyTitle = Nowy klucz odzyskiwania
newRecoveryKeyText = Stary klucz odzyskiwania już nie działa. Zapisz nowy i przechowuj go w bezpiecznym miejscu. Jest wyświetlany tylko teraz.
recoveryReplaced = Klucz odzyskiwania został wymieniony.
or = lub
tryDemo = Wypróbuj na przykładowych danych
tryDemoHint = Przykład jest przechowywany oddzielnie od twoich notatek i można go w każdej chwili całkowicie usunąć.
demoTitle = Przykładowe dane
demoText = Wypróbowujesz ALTAIS na przykładowych notatkach. Gdy będziesz gotowy, usuń przykład i zacznij od własnych.
removeDemo = Usuń przykład i zacznij
removeDemoTitle = Usunąć przykład?
removeDemoText = Przykładowe notatki i ich pliki zostaną całkowicie usunięte. Z przykładu nic nie pozostanie w programie, a twoje własne notatki nie zostaną naruszone.
removeDemoConfirm = Usuń przykład
lock = Zablokuj
saveAs = Zapisz jako…
openAttachment = Otwórz
addAttachment = Dodaj załącznik
cannotLoadAttachment = Nie udało się wczytać załącznika.
language = Język
languageAuto = Jak w systemie
languageHint = Zmiana zostanie zastosowana po ponownym uruchomieniu ALTAIS.

## tools

tool-bold = Pogrubienie (Ctrl+B)
tool-italic = Kursywa (Ctrl+I)
tool-underline = Podkreślenie (Ctrl+U)
tool-strike = Przekreślenie
tool-code = Kod
tool-paragraph = Akapit
tool-heading1 = Nagłówek 1
tool-heading2 = Nagłówek 2
tool-heading3 = Nagłówek 3
tool-bulletList = Lista punktowana
tool-orderedList = Lista numerowana
tool-checkList = Lista zadań
tool-undo = Cofnij (Ctrl+Z)
tool-redo = Ponów (Ctrl+Y)

## errors

error-wrong-password = Nieprawidłowe hasło.
error-wrong-recovery-key = Ten klucz odzyskiwania nie należy do tego archiwum.
error-bad-recovery-key = To nie jest prawidłowy klucz odzyskiwania. Sprawdź, czy nie ma literówek.
error-key-not-on-this-computer = Klucz do archiwum nie jest zapisany na tym komputerze. Użyj hasła lub klucza odzyskiwania.
error-cannot-decrypt = Nie można odszyfrować archiwum: plik jest uszkodzony lub nie pasuje do pliku kluczy.
error-newer-version = Archiwum utworzyła nowsza wersja ALTAIS. Zaktualizuj program.
error-empty-name = Podaj nazwę.
error-demo-only = Niedostępne w przykładzie.
error-damaged-demo-package = Pakiet przykładowych danych jest uszkodzony.
error-missing-demo-package = Brak pakietu przykładowych danych.
error-cannot-remove-demo = Nie udało się usunąć przykładu.
error-empty-password = Hasło nie może być puste.
error-no-protection = Bez hasła archiwum może chronić tylko Windows; w tym systemie nie jest to możliwe.
error-cannot-read-file = Nie można odczytać pliku.
error-cannot-write-file = Nie można zapisać pliku.
error-missing-attachment = Brak załącznika w archiwum.
error-damaged-attachment = Plik załącznika nie istnieje lub jest uszkodzony.
error-unsafe-to-open = Ten plik uruchomiłby się jako program, dlatego ALTAIS go nie otworzy. Możesz go zapisać.
error-cannot-open-file = Nie udało się otworzyć pliku.
error-invalid-note = Notatka zawiera nieprawidłowe dane.
error-unknown = Coś poszło nie tak.
