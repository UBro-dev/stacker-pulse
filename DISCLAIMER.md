# Disclaimer and scope of use

*[Русская версия ниже](#отказ-от-гарантий-и-условия-использования)*

## No warranty

StackerPulse and SeedScan are provided **as is**, with no warranty of any
kind, as stated in [`LICENSE`](LICENSE). Nobody guarantees that the software
is free of defects, that it will run without interruption, or that it is fit
for any particular purpose. You use it at your own risk.

## "No findings" does not mean "you are safe"

SeedScan recognises a fixed set of formats: BIP39 mnemonics, xprv/WIF keys,
Nostr `nsec`, Solana keypairs, Lightning NWC secrets, and operating-system
paths that contain your account name. Everything else it will miss.

It does **not** read archives, PDFs, office documents, images or encrypted
files, it does not recover deleted data, and it cannot see what has already
left your machine. An empty report means only that nothing matched the rules
in this version — it is not a clean bill of health, and it is not evidence
that a device has never been compromised.

## Authorised use only

Run SeedScan on computers and files that **you own, or that you are
explicitly authorised to examine**. Scanning someone else's data without
permission may be unlawful where you live. The tool was written for personal
security hygiene and for professionals working within an agreed scope, and
the author accepts no responsibility for any other use.

## Not an antivirus, not an incident-response tool

The software reads files and reports what it recognises. It removes nothing,
quarantines nothing, repairs nothing, and does not detect malware. If you
suspect that a device has been compromised, treat every secret stored on it
as already lost: move the funds using a **different, clean device** and new
keys. Deleting a file does not reliably erase it from an SSD.

## Not professional advice

Nothing in this software or its documentation is legal, financial, tax or
investment advice. Decisions about your keys and your money are yours.

## Privacy of the software, and of its output

The application performs no telemetry, no analytics and no crash reporting.
Findings are masked and never leave your computer.

The **report you see on screen is a different matter**: it contains file
paths, folder names and, in some places, your operating-system account name.
Treat a saved or shared report as sensitive, and do not publish one without
reading it through first.

## Third-party names

Names such as Windows, Microsoft, Telegram, Discord, Electrum, Exodus,
MetaMask, Phantom, Ledger, Trezor, Alby, GitHub and Stacker News are the
trademarks of their respective owners. They are used here only to describe
which file formats and locations the software examines, or which service it
talks to. No affiliation with, sponsorship by, or endorsement from any of
them is claimed or implied.

## Interoperability with Stacker News

StackerPulse is an independent client. It reads the public GraphQL API of
`stacker.news` on your behalf, at your request, and is neither produced nor
endorsed by Stacker News. You are responsible for using it within the terms
of service of any site you connect to.

---

# Отказ от гарантий и условия использования

## Гарантий нет

StackerPulse и SeedScan распространяются **«как есть»**, без каких-либо
гарантий, как и сказано в [`LICENSE`](LICENSE). Никто не обещает, что
программа работает без ошибок, не прерывается и подходит для какой-то
конкретной задачи. Вы пользуетесь ею на свой риск.

## «Ничего не найдено» не значит «всё чисто»

SeedScan знает ограниченный набор форматов: seed-фразы BIP39, ключи
xprv и WIF, `nsec` в Nostr, ключи Solana, секреты Lightning NWC и пути
операционной системы с вашим именем пользователя. Всё остальное он пропустит.

Он **не** читает архивы, PDF, офисные документы, изображения и зашифрованные
файлы, не восстанавливает удалённое и не видит того, что уже ушло с вашего
компьютера. Пустой отчёт означает лишь то, что под правила этой версии ничего
не подошло. Это не справка о безопасности и не доказательство того, что
устройство никогда не было взломано.

## Только своё и только с разрешения

Запускайте SeedScan на компьютерах и файлах, которые **принадлежат вам либо
которые вам прямо разрешено проверять**. Сканирование чужих данных без
разрешения может быть незаконным в вашей стране. Программа написана для
личной гигиены безопасности и для специалистов, работающих в согласованных
рамках; за любое иное использование автор ответственности не несёт.

## Это не антивирус и не средство реагирования на взлом

Программа читает файлы и сообщает о том, что распознала. Она ничего не
удаляет, не помещает в карантин, не лечит и не обнаруживает вредоносные
программы. Если вы подозреваете, что устройство взломано, считайте все
хранившиеся на нём секреты утраченными: переводите средства **с другого,
заведомо чистого устройства** и на новые ключи. Удаление файла на SSD не
гарантирует, что данные стёрты.

## Это не профессиональная консультация

Ни программа, ни документация к ней не являются юридической, финансовой,
налоговой или инвестиционной консультацией. Решения о своих ключах и деньгах
принимаете вы сами.

## Приватность программы и приватность отчёта

Приложение не собирает телеметрию и статистику, не отправляет отчёты о сбоях.
Находки маскируются и не покидают ваш компьютер.

**С самим отчётом всё иначе**: в нём есть пути к файлам, названия папок, а
местами и имя вашей учётной записи. Относитесь к сохранённому или
пересланному отчёту как к чувствительным данным и не публикуйте его, не
перечитав.

## Чужие названия

Windows, Microsoft, Telegram, Discord, Electrum, Exodus, MetaMask, Phantom,
Ledger, Trezor, Alby, GitHub, Stacker News и другие названия принадлежат их
владельцам. Здесь они используются только для того, чтобы описать, какие
форматы и расположения файлов программа проверяет и к какому сервису
обращается. Ни о какой связи с ними, спонсорстве или одобрении с их стороны
речи не идёт.

## Отношение к Stacker News

StackerPulse — независимый клиент. Он читает публичный GraphQL API
`stacker.news` по вашему запросу и от вашего имени. Stacker News его не
выпускает и не поддерживает. За соблюдение правил сайтов, к которым вы
подключаетесь, отвечаете вы.
