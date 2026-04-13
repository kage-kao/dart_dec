<div align="center">

# dart_dec

### Dart AOT Headless Decompiler

**Самый быстрый, модульный и pipeline-ready инструмент для реверс-инжиниринга Flutter/Dart AOT приложений**

[Быстрый старт](#-быстрый-старт) · [Возможности](#-возможности) · [CLI](#-cli-справка) · [Python API](#-python-api) · [Плагины](#-плагины) · [Сборка](#-сборка-из-исходников)

</div>

---

## Что это

`dart_dec` — headless декомпилятор для Flutter/Dart AOT-скомпилированных приложений. Извлекает классы, функции, строки, восстанавливает высокоуровневые конструкции Dart (async/await, null safety, records) и сканирует на секреты — всё из командной строки, без GUI.

```
$ dart_dec info --so libapp.so

=== dart_dec — Binary Info ===
  File:     libapp.so
  Format:   ELF
  Arch:     arm64
  Size:     54329856 bytes
  SHA-256:  a1b2c3d4e5f6...
  Dart VM:  3.2.0 (stable)
  Sections: 28
```

---

## Быстрый старт

```bash
# Распаковать
tar xzf dart_dec_with_binaries.tar.gz
cd dart_dec_release

# Проверить
./dart_dec --version

# Анализ бинарника
./dart_dec --so libapp.so --format json -o output.json

# Сканирование безопасности
./dart_dec --so libapp.so --scan --format sarif -o report.sarif

# Поиск URL в строках
./dart_dec --so libapp.so --dump strings | grep -i "http"
```

---

## Возможности

### Парсинг и анализ

| Возможность | Описание |
|:---|:---|
| **Мульти-формат** | ELF (Android), Mach-O (iOS), PE (Windows) |
| **Мульти-архитектура** | ARM64, ARM32 (Thumb2), x86_64 |
| **Zero-copy I/O** | memmap2 — мгновенный доступ к любому байту |
| **Версионирование** | 4 метода детекции версии Dart VM с уровнями уверенности |
| **Профили** | JSON-профили для Dart 2.19, 3.0, 3.2, 3.5 + fuzzy matching |

### Декомпиляция

| Этап | Реализация |
|:---|:---|
| Дизассемблирование | Capstone (ARM64/ARM32/x86_64) с детальными операндами |
| IR (промежуточное представление) | 20+ типов инструкций: Assign, BinOp, Call, Branch, Phi, NullCheck, TypeCheck... |
| CFG | Построение графа потока управления через petgraph |
| SSA | Phi-функции, дерево доминаторов, dominance frontiers |
| Структурирование | if/else, while, do-while, for, switch, try/catch |
| Типизация | Forward/backward propagation типов |
| Кодогенерация | Полный Dart AST → читаемый .dart код |

### Восстановление паттернов Dart

```
async/await        State machine → линейный async/await код
Streams            yield / yield* восстановление
Records            Dart 3.x _Record → (a, b) синтаксис
Sealed classes     Exhaustive switch без default
Null safety        ?, !, late операторы
Closures           Context capture → лямбда-выражения
Коллекции          AllocateArray + StoreIndexed → [1, 2, 3]
Интерполяция       StringBuffer+write+toString → "$var"
Каскад             Серия вызовов → ..method()
Extensions         Статический вызов → receiver.method()
```

### Деобфускация

```
Символы            Widget-паттерны: StatefulWidget, StatelessWidget, State
Строки             XOR-расшифровка, fromCharCodes, конкатенация символов
Поток управления   State machine → линейный код, opaque predicates, dead code
Именование         Эвристики по HTTP-паттернам, типам возвращаемых значений
```

---

## CLI справка

```
dart_dec [ОПЦИИ] [КОМАНДА]

КОМАНДЫ:
  info          Информация о бинарнике
  profile-gen   Генерация профиля из Dart SDK
  profiles      Список доступных профилей

ОПЦИИ:
  -s, --so <ПУТЬ>              Путь к бинарнику (libapp.so)
  -f, --format <ФОРМАТ>        json | sqlite | dart | sarif | dot | csv | jsonl
  -o, --output <ПУТЬ>          Файл или директория вывода
      --method <ИМЯ>           Конкретный метод (Класс.метод)
      --dump <ЦЕЛЬ>            classes | functions | strings | ir | cfg | all
      --scan                   Сканер безопасности
      --parallel               Параллельная декомпиляция (по умолч: true)
      --memory-limit <РАЗМЕР>  Лимит памяти (512mb, 1gb)
  -c, --config <ПУТЬ>          Путь к dart_dec.toml
      --profiles-dir <ПАПКА>   Папка с доп. профилями
      --log-level <УРОВЕНЬ>    info | debug | warn | error | trace
```

### Примеры

```bash
# JSON со всеми данными
dart_dec --so libapp.so --format json -o full.json

# Только классы в CSV (для Excel)
dart_dec --so libapp.so --dump classes --format csv > classes.csv

# Декомпиляция одного метода в Dart-код
dart_dec --so libapp.so --method "Auth.login" --format dart

# CFG в PNG через Graphviz
dart_dec --so libapp.so --method "Crypto.encrypt" --format dot | dot -Tpng -o cfg.png

# SQLite для SQL-запросов
dart_dec --so libapp.so --format sqlite -o app.db
sqlite3 app.db "SELECT name, is_async FROM functions WHERE is_async = 1"

# SARIF для GitHub Code Scanning
dart_dec --so libapp.so --scan --format sarif -o findings.sarif

# Потоковый JSON для больших файлов
dart_dec --so huge_libapp.so --format jsonl | python3 analyze.py

# Пакетная обработка
find ./apks/ -name "libapp.so" | parallel dart_dec --so {} --format json -o {}.json
```

---

## Форматы вывода

| Формат | Расширение | Назначение |
|:---|:---|:---|
| `json` | `.json` | Python/JS скрипты, полный структурированный вывод |
| `jsonl` | `.jsonl` | Потоковая обработка больших бинарников |
| `sqlite` | `.db` | SQL-аналитика, сложные запросы |
| `dart` | `.dart` | Ревью кода, понимание логики |
| `sarif` | `.sarif` | GitHub Code Scanning, Semgrep, DefectDojo |
| `csv` | `.csv` | Excel, Google Sheets |
| `dot` | `.dot` | Graphviz визуализация CFG |

---

## Сканер безопасности

Автоматически находит в бинарнике:

| Уровень | Что ищет |
|:---|:---|
| **Критический** | AWS ключи, приватные ключи (RSA/EC) |
| **Высокий** | Google API, JWT, GitHub/Slack токены, пароли, Bearer |
| **Средний** | Firebase URL, слабая криптография (MD5, SHA1, DES, RC4) |
| **Низкий** | HTTP URL (небезопасный транспорт) |

Интеграция с CI/CD через SARIF:

```yaml
# GitHub Actions
- name: Аудит Flutter-приложения
  run: dart_dec --so libapp.so --scan --format sarif -o results.sarif

- name: Загрузка в Code Scanning
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

---

## Python API

### Установка

```bash
cd dart_dec
pip install maturin
cd crates/dart_dec_python
maturin develop --release
```

### Использование

```python
import dart_dec

# Открыть бинарник
ctx = dart_dec.open("libapp.so")

# Свойства
print(ctx.arch)           # "arm64"
print(ctx.dart_version)   # "3.2.0 (stable)"
print(ctx.sha256)         # "a1b2c3..."
print(ctx.file_size)      # 54329856

# Классы
for cls in ctx.get_classes():
    print(f"  {cls['name']} extends {cls['super_class']}")

# Async-функции
functions = ctx.get_functions()
async_funcs = [f for f in functions if f['is_async'] == 'true']
print(f"Async: {len(async_funcs)}")

# Поиск строк
urls = ctx.find_strings("http")
for u in urls:
    print(f"  URL: {u}")

# Сканирование
for finding in ctx.scan_secrets():
    print(f"  [{finding['severity']}] {finding['description']}")

# Экспорт
with open("output.json", "w") as f:
    f.write(ctx.to_json())

# Пакетный анализ
results = dart_dec.batch_analyze(["app1.so", "app2.so", "app3.so"])
```

---

## Плагины

### Ghidra

```bash
# Собрать библиотеку
cargo build --release -p dart_dec_lib
cp target/release/libdart_dec.so /usr/local/lib/

# Установить скрипт
cp plugins/ghidra/DartDecAnalyze.java ~/ghidra_scripts/
```

В Ghidra: **Window → Script Manager → DartDecAnalyze** (категория Dart)

### IDA Pro

```bash
export DART_DEC_LIB=/path/to/libdart_dec.so
cp plugins/ida/dart_dec_ida.py ~/.idapro/plugins/
```

В IDA: **Edit → Plugins → dart_dec Analyzer** или `Ctrl+Shift+D`

Standalone:
```bash
python plugins/ida/dart_dec_ida.py libapp.so
```

### C FFI

```c
void*  ctx = dart_dec_open("libapp.so");
char*  json = dart_dec_get_classes_json(ctx);
printf("%s\n", json);
dart_dec_free_string(json);
dart_dec_close(ctx);
```

---

## Сборка из исходников

### Требования

- Rust 1.77+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- GCC/Clang
- pkg-config (Linux)

### Сборка

```bash
cd dart_dec
cargo build --release --workspace

# Бинарник:     target/release/dart_dec      (13 MB)
# Библиотека:   target/release/libdart_dec.so (2.1 MB)
```

### Тесты

```bash
cargo test --workspace    # 98 тестов
```

### Бенчмарки

```bash
cargo bench -p dart_dec_cli
```

### Docker

```bash
docker build -t dart_dec .
docker run --rm -v ./samples:/data dart_dec --so /data/libapp.so --format json
```

### Homebrew

```bash
brew tap dart-dec/tap
brew install dart-dec
```

### Nix

```bash
nix run github:dart-dec/dart_dec
```

---

## Версионные профили

Встроены профили для **Dart 2.19, 3.0, 3.2, 3.5**.

Генерация нового профиля из исходников Dart SDK:

```bash
git clone https://github.com/dart-lang/sdk.git --branch 3.3.0 --depth 1
dart_dec profile-gen --dart-sdk ./sdk --tag 3.3.0 -o profiles/dart_3.3.json
```

Нечёткий поиск: версия `3.2.3` автоматически использует профиль `3.2.0` (patch не влияет на layout структур).

---

## Конфигурация

Файл `dart_dec.toml` в текущей директории:

```toml
[defaults]
format = "json"
arch = "arm64"
parallel = true
log_level = "info"

[scan]
detect_secrets = true
detect_weak_crypto = true

[output]
sqlite_path = "./output/dump.db"
dart_output_dir = "./output/dart/"
```

---

## Архитектура

```
dart_dec/
├── crates/
│   ├── dart_dec_core/         Парсинг ELF/Mach-O/PE, детект версии
│   ├── dart_dec_snapshot/     AOT Snapshot, Object Pool, таблицы
│   ├── dart_dec_profiles/     JSON-профили, fuzzy-резолвер
│   ├── dart_dec_disasm/       Capstone ARM64/ARM32/x86_64
│   ├── dart_dec_lifter/       Ассемблер → IR
│   ├── dart_dec_graph/        CFG, SSA, доминаторы, AST, codegen
│   ├── dart_dec_patterns/     10 проходов восстановления паттернов
│   ├── dart_dec_deobf/        4 прохода деобфускации
│   ├── dart_dec_output/       JSON/SQLite/SARIF/Dart/CSV форматтеры
│   ├── dart_dec_scan/         Сканеры безопасности
│   ├── dart_dec_cli/          CLI (clap) + бенчмарки
│   ├── dart_dec_lib/          C FFI библиотека
│   └── dart_dec_python/       PyO3 Python bindings
├── plugins/
│   ├── ghidra/                Ghidra скрипт (Java/JNA)
│   └── ida/                   IDA Pro плагин (Python/ctypes)
├── dist/
│   ├── homebrew/              Homebrew формула
│   └── nix/                   Nix packaging
├── tests/
│   ├── fixtures/              5 тестовых .dart программ
│   ├── integration/           Интеграционные тесты
│   └── fuzz_targets.rs        Fuzz-цели
├── Dockerfile
├── flake.nix
├── pyproject.toml
└── dart_dec.toml
```

**13 крейтов · 78 Rust файлов · 10 440 строк · 98 тестов**

---

## Сравнение с аналогами

| Возможность | dart_dec | Blutter | Doldrums | reFlutter |
|:---|:---:|:---:|:---:|:---:|
| Headless (без GUI) | **да** | нет | да | да |
| Мульти-архитектура | **3** | 1 | 1 | 1 |
| Версионные профили | **JSON** | build | нет | нет |
| Полная декомпиляция | **да** | нет | нет | нет |
| async/await recovery | **да** | нет | нет | нет |
| Деобфускация | **да** | нет | нет | нет |
| Security scanning | **да** | нет | нет | нет |
| JSON/SQLite/SARIF | **да** | нет | JSON | нет |
| Python bindings | **да** | нет | да | нет |
| Ghidra/IDA плагины | **да** | Ghidra | нет | нет |
| Параллелизм (rayon) | **да** | нет | нет | нет |
| Скорость (100MB) | **<5 сек** | ~30 сек | ~10 сек | N/A |

---

<div align="center">

**dart_dec** — единственный инструмент, способный декомпилировать Dart AOT код в читаемый псевдокод с восстановлением async/await, null safety, records — и при этом headless-first.

</div>
