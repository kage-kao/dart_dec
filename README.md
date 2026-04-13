================================================================================
 dart_dec — Dart AOT Headless Декомпилятор
 Самый быстрый инструмент реверс-инжиниринга Flutter/Dart AOT приложений
================================================================================

  Версия:   0.1.0
  Язык:     Rust (Edition 2021)
  Платформы: Linux x86_64, macOS (ARM64/x86_64), Windows

================================================================================
 ВОЗМОЖНОСТИ
================================================================================

  ПАРСИНГ
    - ELF (Android libapp.so), Mach-O (iOS libapp.dylib), PE (Windows .exe)
    - Zero-copy memory-mapped I/O (memmap2) — мгновенный доступ к любому байту
    - Определение версии Dart VM (4 метода, уровни уверенности)
    - Полный парсинг AOT Snapshot с восстановлением пула объектов

  МУЛЬТИ-АРХИТЕКТУРА
    - ARM64 (99% Android, все iOS)
    - ARM32 Thumb2 (старые Android устройства)
    - x86_64 (Android эмулятор, десктопный Flutter)

  ДЕКОМПИЛЯЦИЯ
    - Промежуточное представление (IR) с 20+ типами инструкций
    - Построение графа потока управления (CFG)
    - SSA-трансформация (phi-функции, дерево доминаторов)
    - Обнаружение циклов (while, do-while, for, бесконечный)
    - Структурированный вывод (if/else, while, for, switch, try/catch)
    - Распространение типов
    - Полное Dart AST с генерацией кода

  ВОССТАНОВЛЕНИЕ ПАТТЕРНОВ DART
    - async/await — восстановление из state machine
    - Stream API (yield, yield*)
    - Records (Dart 3.x)
    - Sealed classes + exhaustive switch
    - Null safety (?, !, late)
    - Замыкания и лямбды
    - Литералы коллекций (List, Map, Set)
    - Строковая интерполяция ("$переменная")
    - Каскадный оператор (..)
    - Extension методы

  ДЕОБФУСКАЦИЯ
    - Восстановление имен символов (паттерны Flutter виджетов)
    - Расшифровка строк (XOR, fromCharCodes, конкатенация символов)
    - Расплющивание потока управления (state machine -> линейный код)
    - Удаление непрозрачных предикатов
    - Эвристическое именование
    - Удаление мертвого кода

  ФОРМАТЫ ВЫВОДА
    - JSON (полный структурированный)
    - JSON Lines (потоковый для больших бинарников)
    - SQLite (полная схема с индексами)
    - SARIF v2.1 (GitHub Code Scanning, Semgrep)
    - Dart исходный код (best-effort)
    - CSV (для таблиц)
    - DOT (Graphviz визуализация CFG)

  СКАНЕР БЕЗОПАСНОСТИ
    - AWS ключи, Google API ключи, JWT токены
    - GitHub/Slack/Bearer токены
    - Захардкоженные пароли, приватные ключи
    - HTTP URL (небезопасный транспорт)
    - Слабая криптография (MD5, SHA1, DES, RC4, ECB)
    - Анализ Android permissions
    - SARIF вывод для CI/CD интеграции

  ПРОИЗВОДИТЕЛЬНОСТЬ
    - Параллельная декомпиляция через rayon (work-stealing)
    - Арена-аллокация bumpalo (5x быстрее стандартного аллокатора)
    - Изоляция паник на уровне функций (без крашей)
    - Прогресс-бары с ETA
    - Ctrl+C — корректное завершение
    - Поддержка лимита памяти

  ИНТЕГРАЦИИ
    - C FFI библиотека (libdart_dec.so/.dylib)
    - Python bindings (PyO3, pip install)
    - Ghidra плагин (JNA мост)
    - IDA Pro плагин (ctypes мост)
    - Docker контейнер
    - Homebrew формула
    - Nix Flake

================================================================================
 БЫСТРЫЙ СТАРТ
================================================================================

  # Информация о бинарнике
  ./dart_dec info --so libapp.so

  # Полная декомпиляция в JSON
  ./dart_dec --so libapp.so --format json -o output.json

  # Сканирование безопасности в SARIF
  ./dart_dec --so libapp.so --scan --format sarif -o report.sarif

  # Все строки (поиск URL)
  ./dart_dec --so libapp.so --dump strings | grep -i "http"

  # Дамп классов в CSV
  ./dart_dec --so libapp.so --dump classes --format csv

  # Декомпиляция конкретного метода
  ./dart_dec --so libapp.so --method "Auth.login" --format dart

  # SQLite база для SQL-анализа
  ./dart_dec --so libapp.so --format sqlite -o analysis.db

  # CFG визуализация
  ./dart_dec --so libapp.so --method "Pay.process" --format dot | dot -Tpng -o cfg.png

  # Пакетная обработка
  find ./samples/ -name "libapp.so" | parallel ./dart_dec --so {} --format json -o {}.json

================================================================================
 СПРАВКА ПО CLI
================================================================================

  ИСПОЛЬЗОВАНИЕ:
    dart_dec [ОПЦИИ] [КОМАНДА]

  КОМАНДЫ:
    info          Информация о бинарнике (архитектура, версия, секции)
    profile-gen   Генерация профиля из исходников Dart SDK
    profiles      Список доступных профилей Dart VM

  ОПЦИИ:
    -s, --so <ПУТЬ>            Путь к AOT бинарнику (libapp.so)
    -f, --format <ФОРМАТ>      json|sqlite|dart|sarif|dot|csv|jsonl
    -o, --output <ПУТЬ>        Файл или директория для вывода
        --method <ИМЯ>         Конкретный метод (Класс.метод)
        --dump <ЦЕЛЬ>          classes|functions|strings|ir|cfg|all
        --scan                 Запустить сканер безопасности
        --parallel             Параллельная декомпиляция (по умолч.: true)
        --memory-limit <РАЗМЕР> Лимит памяти (512mb, 1gb)
    -c, --config <ПУТЬ>        Путь к dart_dec.toml
        --profiles-dir <ПАПКА> Папка с дополнительными профилями
        --log-level <УРОВЕНЬ>  info|debug|warn|error|trace

================================================================================
 PYTHON API
================================================================================

  import dart_dec

  ctx = dart_dec.open("libapp.so")

  # Свойства
  ctx.arch            # "arm64"
  ctx.dart_version    # "3.2.0 (stable)"
  ctx.sha256          # "abc123..."
  ctx.file_size       # 52428800

  # Извлечение данных
  ctx.get_classes()      # [{"name": "MyApp", "super_class": "StatelessWidget"}]
  ctx.get_functions()    # [{"name": "build", "kind": "regular", "is_async": "false"}]
  ctx.get_strings()      # ["Hello", "https://api.example.com", ...]
  ctx.find_strings("api") # поиск по подстроке

  # Безопасность
  ctx.scan_secrets()     # [{"severity": "Critical", "description": "Найден AWS ключ"}]

  # Экспорт
  ctx.to_json()          # полная JSON строка

  # Пакетный анализ
  dart_dec.batch_analyze(["app1.so", "app2.so"])
  dart_dec.available_profiles()  # ["2.19.0", "3.0.0", "3.2.0", "3.5.0"]

================================================================================
 C FFI API
================================================================================

  void*  dart_dec_open(const char* path);
  char*  dart_dec_get_classes_json(void* ctx);
  char*  dart_dec_get_strings_json(void* ctx);
  char*  dart_dec_decompile_function(void* ctx, const char* cls, const char* func);
  void   dart_dec_free_string(char* ptr);
  void   dart_dec_close(void* ctx);

================================================================================
 ПОДДЕРЖИВАЕМЫЕ ВЕРСИИ DART
================================================================================

  Встроенные профили:
    - Dart 2.19.x (последняя Dart 2)
    - Dart 3.0.x  (null safety, records)
    - Dart 3.2.x  (sealed classes)
    - Dart 3.5.x  (последние фичи)

  Генерация нового: dart_dec profile-gen --dart-sdk /путь/к/sdk --tag 3.3.0

  Нечёткий поиск: 3.2.3 автоматически использует профиль 3.2.0

================================================================================
 СТРУКТУРА ПРОЕКТА (13 крейтов, 78 Rust файлов, 10440 строк)
================================================================================

  dart_dec_core       Парсинг ELF/Mach-O/PE, детект версии
  dart_dec_snapshot   AOT Snapshot, Object Pool, таблицы классов/строк
  dart_dec_profiles   Версионные профили, fuzzy-резолвер
  dart_dec_disasm     Capstone ARM64/ARM32/x86_64
  dart_dec_lifter     Ассемблер -> IR (лифтинг)
  dart_dec_graph      CFG, SSA, доминаторы, структурирование, AST
  dart_dec_patterns   Восстановление паттернов Dart (10 проходов)
  dart_dec_deobf      Деобфускация (4 прохода)
  dart_dec_output     Форматтеры JSON/SQLite/SARIF/Dart/CSV
  dart_dec_scan       Сканеры безопасности
  dart_dec_cli        CLI точка входа (clap)
  dart_dec_lib        C FFI библиотека
  dart_dec_python     PyO3 Python bindings

================================================================================
 ТЕСТИРОВАНИЕ
================================================================================

  98 unit-тестов по всем крейтам
  Интеграционные тесты (полный пайплайн IR -> AST -> Dart)
  Criterion бенчмарки (6 бенчмарков)
  Fuzz-тесты (4 цели через cargo-fuzz)
  5 тестовых Dart-фикстур (простой класс, async, sealed, null safety, коллекции)

  Запуск: cargo test --workspace

================================================================================
