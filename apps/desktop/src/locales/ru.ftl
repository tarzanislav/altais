# ALTAIS – тексты интерфейса (русский). Формат: Project Fluent.

appName = ALTAIS
loading = Загрузка…
welcomeTitle = Добро пожаловать в ALTAIS
welcomeText = Заметки будут храниться в зашифрованном архиве на этом компьютере. Ключ к нему надёжно сохранит Windows, поэтому пароль не нужен. При желании пароль можно включить сразу.
archiveFolder = Папка архива
usePassword = Защитить архив ещё и паролем
passwordRequired = Эта система не умеет хранить ключ, поэтому пароль обязателен.
password = Пароль
passwordAgain = Повторите пароль
passwordsDiffer = Пароли не совпадают.
createArchive = Создать архив
recoveryTitle = Ваш ключ восстановления
recoveryText = Этим ключом можно открыть архив на другом компьютере или если вы забудете пароль. Запишите или распечатайте его и храните в надёжном месте. Он показывается только сейчас.
copy = Копировать
copied = Скопировано
recoverySaved = Я надёжно сохранил ключ восстановления
continue = Продолжить
unlockTitle = Открыть архив
unlockWithPassword = Паролем
unlockWithRecovery = Ключом восстановления
recoveryKey = Ключ восстановления
open = Открыть
allNotes = Все заметки
notebooks = Блокноты
newNotebook = Новый блокнот
notebookName = Название блокнота
create = Создать
cancel = Отмена
importEvernote = Импорт из Evernote
importing = Импорт…
importDone = Импорт завершён
importSummary = { $notes ->
        [one] { $notes } заметка
        [few] { $notes } заметки
       *[other] { $notes } заметок
    } и { $attachments ->
        [one] { $attachments } вложение
        [few] { $attachments } вложения
       *[other] { $attachments } вложений
    } в блокнот «{ $notebook }»
partlyImported = Импортировано частично:
stoppedEarly = Файл не удалось дочитать до конца:
close = Закрыть
search = Поиск…
noNotes = Нет заметок
noResults = Ничего не найдено
newNote = Новая заметка
untitled = Без названия
titlePlaceholder = Название
saving = Сохранение…
saved = Сохранено
saveFailed = Не удалось сохранить
trash = В корзину
selectNote = Выберите заметку или создайте новую.
settings = Настройки
passwordOn = Архив защищён паролем. ALTAIS запросит его при открытии.
passwordOff = Пароль не задан. Ключ к архиву надёжно хранит Windows.
setPassword = Задать пароль
changePassword = Изменить пароль
removePassword = Удалить пароль
newPassword = Новый пароль
savePassword = Сохранить пароль
passwordSet = Пароль задан.
passwordRemoved = Пароль удалён. Ключ снова хранит Windows.
recoveryKeyHint = Если вы потеряли старый ключ восстановления, создайте новый. Старый перестанет работать.
createNewRecoveryKey = Создать новый ключ восстановления
newRecoveryKeyTitle = Новый ключ восстановления
newRecoveryKeyText = Старый ключ восстановления больше не работает. Запишите новый и храните его в надёжном месте. Он показывается только сейчас.
recoveryReplaced = Ключ восстановления заменён.
or = или
tryDemo = Попробовать на примере
tryDemoHint = Пример хранится отдельно от ваших заметок, и его можно в любой момент полностью удалить.
demoTitle = Пример
demoText = Вы пробуете ALTAIS на примерах заметок. Когда будете готовы, удалите пример и начните со своих.
removeDemo = Удалить пример и начать
removeDemoTitle = Удалить пример?
removeDemoText = Заметки примера и их файлы будут полностью удалены. От примера в программе ничего не останется, а ваши собственные заметки не пострадают.
removeDemoConfirm = Удалить пример
lock = Заблокировать
saveAs = Сохранить как…
openAttachment = Открыть
addAttachment = Добавить вложение
cannotLoadAttachment = Не удалось загрузить вложение.
language = Язык
languageAuto = Как в системе
languageHint = Изменение вступит в силу после перезапуска ALTAIS.
trashTitle = Корзина
trashEmpty = Корзина пуста
emptyTrash = Очистить корзину
emptyTrashTitle = Очистить корзину?
emptyTrashText = { $count ->
        [one] { $count } заметка будет удалена навсегда вместе с вложениями, которые не используются в других заметках. Это нельзя отменить.
        [few] { $count } заметки будут удалены навсегда вместе с вложениями, которые не используются в других заметках. Это нельзя отменить.
       *[other] { $count } заметок будут удалены навсегда вместе с вложениями, которые не используются в других заметках. Это нельзя отменить.
    }
inTrash = Эта заметка в корзине.
restore = Восстановить
deleteForever = Удалить навсегда
deleteForeverTitle = Удалить заметку навсегда?
deleteForeverText = Заметка будет удалена вместе с вложениями, которые не используются в других заметках. Это нельзя отменить.

## tools

tool-bold = Жирный (Ctrl+B)
tool-italic = Курсив (Ctrl+I)
tool-underline = Подчёркнутый (Ctrl+U)
tool-strike = Зачёркнутый
tool-code = Код
tool-paragraph = Абзац
tool-heading1 = Заголовок 1
tool-heading2 = Заголовок 2
tool-heading3 = Заголовок 3
tool-bulletList = Маркированный список
tool-orderedList = Нумерованный список
tool-checkList = Список задач
tool-undo = Отменить (Ctrl+Z)
tool-redo = Повторить (Ctrl+Y)

## errors

error-wrong-password = Неверный пароль.
error-wrong-recovery-key = Этот ключ восстановления не относится к этому архиву.
error-bad-recovery-key = Это недействительный ключ восстановления. Проверьте, нет ли опечаток.
error-key-not-on-this-computer = Ключ к архиву не сохранён на этом компьютере. Используйте пароль или ключ восстановления.
error-cannot-decrypt = Не удаётся расшифровать архив: файл повреждён или не соответствует файлу ключей.
error-newer-version = Архив создан более новой версией ALTAIS. Обновите программу.
error-empty-name = Введите название.
error-demo-only = В примере недоступно.
error-damaged-demo-package = Пакет примера повреждён.
error-missing-demo-package = Пакет примера отсутствует.
error-cannot-remove-demo = Не удалось удалить пример.
error-empty-password = Пароль не может быть пустым.
error-no-protection = Без пароля архив может защитить только Windows; в этой системе это невозможно.
error-cannot-read-file = Не удаётся прочитать файл.
error-cannot-write-file = Не удаётся сохранить файл.
error-missing-attachment = Вложение отсутствует в архиве.
error-damaged-attachment = Файл вложения отсутствует или повреждён.
error-unsafe-to-open = Этот файл запустился бы как программа, поэтому ALTAIS его не откроет. Его можно сохранить.
error-cannot-open-file = Не удалось открыть файл.
error-invalid-note = Заметка содержит недопустимые данные.
error-not-in-trash = Навсегда можно удалить только заметки из корзины.
error-unknown = Что-то пошло не так.
