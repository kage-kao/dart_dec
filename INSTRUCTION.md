<div align="center">

# Инструкция по dart_dec

### Полное руководство по установке, настройке и использованию

</div>

---

## Содержание

1. [Что это такое](#1-что-это-такое)
2. [Системные требования](#2-системные-требования)
3. [Установка из готового бинарника](#3-установка-из-готового-бинарника)
4. [Сборка из исходников](#4-сборка-из-исходников)
5. [Основные команды CLI](#5-основные-команды-cli)
6. [Форматы вывода](#6-форматы-вывода)
7. [Конфигурация dart_dec.toml](#7-конфигурация-dart_dectoml)
8. [Python bindings](#8-python-bindings)
9. [Ghidra Plugin](#9-ghidra-plugin)
10. [IDA Pro Plugin](#10-ida-pro-plugin)
11. [C FFI библиотека](#11-c-ffi-библиотека-libdart_decso)
12. [Docker](#12-docker)
13. [Homebrew и Nix](#13-homebrew-и-nix)
14. [Сканер безопасности](#14-сканер-безопасности)
15. [Пакетная обработка](#15-пакетная-обработка)
16. [Версионные профили](#16-версионные-профили)
17. [Примеры реальных сценариев](#17-примеры-реальных-сценариев)
18. [Устранение неполадок](#18-устранение-неполадок)
19. [Структура проекта](#19-структура-проекта)

---

## 1. Что это такое

`dart_dec` — декомпилятор для Flutter/Dart AOT-скомпилированных приложений. Работает из командной строки (headless), подходит для CI/CD пайплайнов, массового анализа и интеграции в другие инструменты.

**Что умеет:**

- Парсить `libapp.so` (Android), `libapp.dylib` (iOS), `.exe` (Windows desktop)
- Определять версию Dart VM (4 метода детекции)
- Извлекать классы, функции, строки, типы из AOT Snapshot
- Дизассемблировать ARM64, ARM32, x86_64
- Поднимать ассемблер до промежуточного представления (IR)
- Строить граф потока управления (CFG) и SSA-форму
- Декомпилировать в читаемый Dart-подобный код
- Восстанавливать async/await, null safety, closures, records
- Деобфусцировать (восстановление имён, расшифровка строк)
- Сканировать на секреты (API ключи, токены, пароли)
- Выводить в JSON, SQLite, SARIF, Dart, CSV, DOT

---

## 2. Системные требования

### Для использования готового бинарника

| Платформа | Версия |
|:---|:---|
| Linux x86_64 | Ubuntu 20.04+, Debian 11+, Fedora 36+ |
| macOS | 12+ (при самостоятельной сборке) |
| Windows | 10+ (при самостоятельной сборке) |

### Для сборки из исходников

| Компонент | Версия |
|:---|:---|
| Rust | 1.77+ ([rustup.rs](https://rustup.rs)) |
| GCC/Clang | любая современная |
| pkg-config | (только Linux) |
| Свободное место | 2 GB для сборки |

### Для Python bindings

| Компонент | Версия |
|:---|:---|
| Python | 3.8+ |
| maturin | `pip install maturin` |

---

## 3. Установка из готового бинарника

### Распаковка

```bash
tar xzf dart_dec_with_binaries.tar.gz
cd dart_dec_release
```

### Проверка

```bash
./dart_dec --version
# dart_dec 0.1.0

./dart_dec profiles
# Available Dart VM profiles:
#   - Dart 2.19.0
#   - Dart 3.0.0
#   - Dart 3.2.0
#   - Dart 3.5.0
```

### Установка в систему (опционально)

```bash
sudo cp dart_dec /usr/local/bin/
sudo cp libdart_dec.so /usr/local/lib/
sudo ldconfig
```

---

## 4. Сборка из исходников

### Шаг 1: Установка Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Шаг 2: Сборка

```bash
cd dart_dec
cargo build --release --workspace
```

Результат:

| Файл | Путь | Размер |
|:---|:---|:---|
| CLI бинарник | `target/release/dart_dec` | ~13 MB |
| C FFI библиотека | `target/release/libdart_dec.so` | ~2.1 MB |

### Шаг 3: Тесты

```bash
cargo test --workspace
# 98 tests, 0 failures
```

### Шаг 4: Бенчмарки

```bash
cargo bench -p dart_dec_cli
```

---

## 5. Основные команды CLI

### Информация о бинарнике

```bash
dart_dec info --so libapp.so
```

Выводит формат, архитектуру, версию Dart VM, список секций, SHA-256 хэш.

### Полная декомпиляция в JSON

```bash
dart_dec --so libapp.so --format json -o output.json
```

### Дамп классов в CSV

```bash
dart_dec --so libapp.so --dump classes --format csv
```

### Дамп строк

```bash
dart_dec --so libapp.so --dump strings | grep -i "http"
```

### Декомпиляция конкретного метода

```bash
dart_dec --so libapp.so --method "Auth.login" --format json
```

### Генерация Dart-кода

```bash
dart_dec --so libapp.so --format dart -o output/
```

### Security scan

```bash
dart_dec --so libapp.so --scan --format sarif -o report.sarif
```

### CFG визуализация

```bash
dart_dec --so libapp.so --method "Payment.process" --format dot | dot -Tpng -o cfg.png
```

### SQLite база для аналитики

```bash
dart_dec --so libapp.so --format sqlite -o analysis.db
sqlite3 analysis.db "SELECT name FROM classes ORDER BY name"
```

### Список профилей

```bash
dart_dec profiles
```

### Полная справка

```bash
dart_dec --help
```

### Все флаги

| Флаг | Описание | По умолчанию |
|:---|:---|:---|
| `--so <ПУТЬ>` | Путь к бинарнику | — |
| `--format <FMT>` | json, sqlite, dart, sarif, dot, csv, jsonl | json |
| `--output <ПУТЬ>` | Файл/директория вывода | stdout |
| `--method <ИМЯ>` | Конкретный метод (Класс.метод) | все |
| `--dump <ЦЕЛЬ>` | classes, functions, strings, ir, cfg, all | — |
| `--scan` | Запуск сканера безопасности | false |
| `--parallel` | Параллельная декомпиляция | true |
| `--memory-limit` | Лимит памяти (512mb, 1gb) | без лимита |
| `--config <ПУТЬ>` | Путь к dart_dec.toml | ./dart_dec.toml |
| `--profiles-dir` | Папка с доп. профилями | встроенные |
| `--log-level` | info, debug, warn, error, trace | info |

---

## 6. Форматы вывода

| Формат | Для чего | Пример |
|:---|:---|:---|
| **json** | Python/JS скрипты, автоматизация | `--format json -o out.json` |
| **jsonl** | Потоковая обработка больших файлов | `--format jsonl \| python3 analyze.py` |
| **sqlite** | SQL-аналитика, сложные запросы | `--format sqlite -o app.db` |
| **dart** | Ревью кода, понимание логики | `--format dart -o output/` |
| **sarif** | GitHub Code Scanning, Semgrep | `--scan --format sarif` |
| **csv** | Excel, Google Sheets | `--dump classes --format csv` |
| **dot** | Graphviz CFG визуализация | `--format dot \| dot -Tpng` |

---

## 7. Конфигурация dart_dec.toml

Создай файл `dart_dec.toml` в рабочей директории:

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

dart_dec подхватит автоматически, или укажи явно:

```bash
dart_dec --config ./my_config.toml --so libapp.so
```

---

## 8. Python bindings

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
print(ctx.sha256)         # "abc123..."
print(ctx.file_size)      # 52428800
print(ctx.format)         # "ELF"

# Классы
classes = ctx.get_classes()
for cls in classes:
    print(f"  {cls['name']} extends {cls['super_class']}")

# Функции
functions = ctx.get_functions()
async_funcs = [f for f in functions if f['is_async'] == 'true']
print(f"Async функций: {len(async_funcs)}")

# Строки
strings = ctx.get_strings()
urls = [s for s in strings if s.startswith("http")]
print(f"URL найдено: {len(urls)}")

# Поиск строк по подстроке
api_strings = ctx.find_strings("api")

# Сканирование безопасности
findings = ctx.scan_secrets()
for f in findings:
    print(f"  [{f['severity']}] {f['description']}")

# JSON экспорт
json_str = ctx.to_json()

# Пакетный анализ нескольких файлов
results = dart_dec.batch_analyze(["app1.so", "app2.so", "app3.so"])
for r in results:
    print(f"  {r['path']}: {r['status']} arch={r.get('arch', '?')}")

# Доступные профили
profiles = dart_dec.available_profiles()
print(profiles)  # ['2.19.0', '3.0.0', '3.2.0', '3.5.0']
```

---

## 9. Ghidra Plugin

### Установка

```bash
# 1. Собрать библиотеку
cargo build --release -p dart_dec_lib

# 2. Скопировать библиотеку
sudo cp target/release/libdart_dec.so /usr/local/lib/

# 3. Скопировать скрипт в Ghidra
cp plugins/ghidra/DartDecAnalyze.java ~/ghidra_scripts/

# 4. (опционально) Задать переменную окружения
export DART_DEC_LIB=/usr/local/lib/libdart_dec.so
```

### Использование

1. Открой `libapp.so` в Ghidra
2. **Window → Script Manager**
3. Найди **DartDecAnalyze** в категории **Dart**
4. Запусти

**Результат:**
- Классы аннотируются Dart-именами
- URL и секреты выводятся в консоль
- Функции получают комментарии

---

## 10. IDA Pro Plugin

### Установка

```bash
# 1. Собрать библиотеку
cargo build --release -p dart_dec_lib

# 2. Задать путь
export DART_DEC_LIB=/path/to/libdart_dec.so

# 3. Скопировать плагин
cp plugins/ida/dart_dec_ida.py ~/.idapro/plugins/
```

### Использование в IDA

- **Edit → Plugins → dart_dec Analyzer**
- Или горячая клавиша: **Ctrl+Shift+D**

### Из IDAPython консоли

```python
import dart_dec_ida
dart_dec_ida.run()
```

### Standalone (без IDA)

```bash
python plugins/ida/dart_dec_ida.py libapp.so
```

---

## 11. C FFI библиотека (libdart_dec.so)

### API

```c
// Открыть бинарник → контекст
void* dart_dec_open(const char* path);

// Получить классы (JSON массив)
char* dart_dec_get_classes_json(void* ctx);

// Получить строки (JSON массив)
char* dart_dec_get_strings_json(void* ctx);

// Декомпилировать функцию
char* dart_dec_decompile_function(void* ctx, const char* class, const char* func);

// Освободить строку
void dart_dec_free_string(char* ptr);

// Закрыть контекст
void dart_dec_close(void* ctx);
```

### Пример на C

```c
#include <stdio.h>
#include <dlfcn.h>

int main() {
    void* lib = dlopen("libdart_dec.so", RTLD_NOW);

    void* (*open_fn)(const char*) = dlsym(lib, "dart_dec_open");
    char* (*classes_fn)(void*)    = dlsym(lib, "dart_dec_get_classes_json");
    void  (*free_fn)(char*)       = dlsym(lib, "dart_dec_free_string");
    void  (*close_fn)(void*)      = dlsym(lib, "dart_dec_close");

    void* ctx = open_fn("libapp.so");
    char* json = classes_fn(ctx);
    printf("%s\n", json);
    free_fn(json);
    close_fn(ctx);
    dlclose(lib);
}
```

### Пример на Python (ctypes)

```python
import ctypes, json

lib = ctypes.CDLL("libdart_dec.so")
lib.dart_dec_open.restype = ctypes.c_void_p
lib.dart_dec_get_classes_json.restype = ctypes.c_void_p

ctx = lib.dart_dec_open(b"libapp.so")
ptr = lib.dart_dec_get_classes_json(ctx)
data = json.loads(ctypes.string_at(ptr).decode())
lib.dart_dec_free_string(ptr)
lib.dart_dec_close(ctx)

for cls in data:
    print(cls["name"])
```

---

## 12. Docker

### Сборка образа

```bash
cd dart_dec
docker build -t dart_dec .
```

### Использование

```bash
# Анализ файла
docker run --rm -v ./samples:/data dart_dec --so /data/libapp.so --format json

# Security scan
docker run --rm -v ./samples:/data dart_dec --so /data/libapp.so --scan

# SQLite экспорт
docker run --rm -v ./samples:/data dart_dec \
    --so /data/libapp.so --format sqlite -o /data/output.db
```

---

## 13. Homebrew и Nix

### Homebrew (macOS/Linux)

```bash
brew tap dart-dec/tap
brew install dart-dec
```

### Nix

```bash
# Запуск без установки
nix run github:dart-dec/dart_dec

# Установка
nix profile install github:dart-dec/dart_dec

# Dev shell (для разработки)
nix develop github:dart-dec/dart_dec
```

### Cargo install

```bash
cargo install --git https://github.com/dart-dec/dart_dec dart_dec_cli
```

---

## 14. Сканер безопасности

### Что ищет

| Уровень | Паттерн | Пример |
|:---|:---|:---|
| **Критический** | AWS Access Key | `AKIAIOSFODNN7EXAMPLE` |
| **Критический** | AWS Secret Key | длинный base64 после `aws` |
| **Критический** | Приватные ключи | `-----BEGIN RSA PRIVATE KEY-----` |
| **Высокий** | Google API Key | `AIzaSy...` |
| **Высокий** | JWT Token | `eyJhbG...` |
| **Высокий** | Slack Token | `xoxb-...` |
| **Высокий** | GitHub Token | `ghp_...` |
| **Высокий** | Захардкоженные пароли | `password = "..."` |
| **Высокий** | Bearer Token | `Bearer eyJ...` |
| **Средний** | Firebase URL | `https://xxx.firebaseio.com` |
| **Средний** | Слабая криптография | MD5, SHA1, DES, RC4, ECB |
| **Низкий** | HTTP URL | `http://...` (без TLS) |

### Вывод в SARIF для CI/CD

```yaml
# GitHub Actions
- name: Аудит Flutter-приложения
  run: dart_dec --so libapp.so --scan --format sarif -o results.sarif

- name: Загрузка в Code Scanning
  uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: results.sarif
```

Совместимость:
- GitHub Code Scanning
- Semgrep
- OWASP DefectDojo
- Azure DevOps

---

## 15. Пакетная обработка

### Shell

```bash
# Все libapp.so в директории
find ./samples/ -name "libapp.so" | while read f; do
    dart_dec --so "$f" --format json -o "${f}.json"
done
```

### GNU Parallel

```bash
find ./samples/ -name "libapp.so" | \
    parallel dart_dec --so {} --format json -o {}.json
```

### Python

```python
import dart_dec

results = dart_dec.batch_analyze([
    "samples/app1/libapp.so",
    "samples/app2/libapp.so",
    "samples/app3/libapp.so",
])

for r in results:
    print(f"{r['path']}: {r['status']}, Dart {r.get('dart_version', '?')}")
```

---

## 16. Версионные профили

dart_dec поставляется с профилями:

| Версия | Особенности |
|:---|:---|
| Dart 2.19 | Последняя Dart 2 |
| Dart 3.0 | Null safety, records |
| Dart 3.2 | Sealed classes |
| Dart 3.5 | Последние фичи |

### Автогенерация нового профиля

```bash
# Скачать Dart SDK
git clone https://github.com/dart-lang/sdk.git --branch 3.3.0 --depth 1

# Сгенерировать профиль
dart_dec profile-gen --dart-sdk ./sdk --tag 3.3.0 -o profiles/dart_3.3.json
```

### Указать папку с дополнительными профилями

```bash
dart_dec --so libapp.so --profiles-dir ./my_profiles/ --format json
```

### Нечёткий поиск (fuzzy matching)

Если точной версии нет, dart_dec подберёт ближайшую автоматически:

| Запрошена | Используется | Комментарий |
|:---|:---|:---|
| 3.2.3 | 3.2.0 | patch не влияет на layout |
| 3.1.5 | ближайший 3.x | с предупреждением |
| 4.0.0 | — | ABORT + инструкция по генерации |

---

## 17. Примеры реальных сценариев

### Аудит мобильного приложения

```bash
# Извлечь libapp.so из APK
unzip app.apk -d extracted
cp extracted/lib/arm64-v8a/libapp.so .

# Полный анализ + сканирование
dart_dec --so libapp.so --scan --format json -o audit.json

# Все URL
dart_dec --so libapp.so --dump strings | grep -i "http"

# SARIF для CI
dart_dec --so libapp.so --scan --format sarif -o findings.sarif
```

### Малварь-анализ

```bash
# Быстрый обзор
dart_dec info --so malware_libapp.so

# SQLite для детального анализа
dart_dec --so malware_libapp.so --format sqlite -o malware.db

# SQL-запросы
sqlite3 malware.db "SELECT value FROM strings WHERE value LIKE '%http%'"
sqlite3 malware.db "SELECT name, is_async FROM functions WHERE is_async = 1"
sqlite3 malware.db "SELECT * FROM security_findings ORDER BY severity"
```

### CTF / Reverse Engineering

```bash
# Декомпиляция конкретной функции
dart_dec --so challenge.so --method "Flag.getFlag" --format dart

# CFG визуализация
dart_dec --so challenge.so --method "Crypto.encrypt" --format dot > cfg.dot
dot -Tpng cfg.dot -o cfg.png
```

### CI/CD интеграция

```yaml
# .github/workflows/audit.yml
name: Flutter Security Audit
on: [push]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build Flutter app
        run: flutter build apk --release

      - name: Extract libapp.so
        run: unzip build/app/outputs/flutter-apk/app-release.apk -d extracted

      - name: Run dart_dec scan
        run: |
          dart_dec --so extracted/lib/arm64-v8a/libapp.so \
            --scan --format sarif -o results.sarif

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: results.sarif
```

---

## 18. Устранение неполадок

### "Could not detect Dart version"

Символы могут быть stripped. dart_dec попробует эвристику (fingerprinting).

**Решение:** Укажи профиль вручную:
```bash
dart_dec --so libapp.so --profiles-dir ./profiles/ --format json
```

### "No profile found for Dart X.Y.Z"

**Решение:** Сгенерируй профиль:
```bash
dart_dec profile-gen --dart-sdk /path/to/sdk --tag X.Y.Z -o profiles/dart_X.Y.json
```

Или dart_dec подберёт ближайший автоматически.

### "Capstone init failed"

Capstone встроена в бинарник (bundled). Если ошибка при сборке:

```bash
# Ubuntu/Debian
apt install libcapstone-dev

# macOS
brew install capstone
```

### "Permission denied"

```bash
chmod +x dart_dec
```

### Медленная обработка

- `--parallel` включён по умолчанию
- Для больших файлов: `--format jsonl` (потоковый вывод)
- Лимит памяти: `--memory-limit 512mb`

### Крэш на конкретном бинарнике

dart_dec изолирует паники на уровне функций — если одна функция падает, остальные продолжают работать. В отчёте будет `coverage_%` показывающий сколько функций декомпилировано.

---

## 19. Структура проекта

```
dart_dec/
├── Cargo.toml                  Workspace root
├── Dockerfile                  Docker image
├── README.md                   Основной README
├── dart_dec.toml               Пример конфигурации
├── flake.nix                   Nix Flake
├── pyproject.toml              Python packaging (maturin)
│
├── crates/
│   ├── dart_dec_core/          ELF/Mach-O/PE парсинг, детект версии
│   ├── dart_dec_snapshot/      AOT Snapshot, Object Pool
│   ├── dart_dec_profiles/      JSON-профили версий Dart
│   ├── dart_dec_disasm/        Capstone ARM64/ARM32/x86_64
│   ├── dart_dec_lifter/        Ассемблер → IR
│   ├── dart_dec_graph/         CFG, SSA, AST, Dart codegen
│   ├── dart_dec_patterns/      async/await, closures, records...
│   ├── dart_dec_deobf/         Деобфускация
│   ├── dart_dec_output/        JSON/SQLite/SARIF/Dart/CSV
│   ├── dart_dec_scan/          Сканер безопасности
│   ├── dart_dec_cli/           CLI (clap) + бенчмарки
│   ├── dart_dec_lib/           C FFI (.so/.dylib)
│   └── dart_dec_python/        PyO3 Python bindings
│
├── plugins/
│   ├── ghidra/                 Ghidra скрипт (Java/JNA)
│   │   ├── DartDecAnalyze.java
│   │   └── README.md
│   └── ida/                    IDA Pro плагин (Python/ctypes)
│       ├── dart_dec_ida.py
│       └── README.md
│
├── dist/
│   ├── homebrew/               Homebrew формула
│   │   ├── dart-dec.rb
│   │   └── README.md
│   └── nix/                    Nix packaging
│
├── tests/
│   ├── fixtures/               5 тестовых .dart программ
│   │   ├── simple_class.dart
│   │   ├── async_function.dart
│   │   ├── sealed_class.dart
│   │   ├── null_safety.dart
│   │   └── collections.dart
│   ├── integration/            Интеграционные тесты
│   ├── regression/             Регрессионные тесты
│   └── fuzz_targets.rs         Fuzz-цели
│
├── docs/                       Документация
└── .github/workflows/          CI/CD
    └── ci.yml                  Test + Lint + Release (4 платформы)
```

**13 крейтов · 78 Rust файлов · 10 440 строк · 98 тестов · 116 файлов**
