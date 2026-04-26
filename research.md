# Дослідження: Абстрагування виклику AI-команди для перекладу

## Тема дослідження

Абстрагування зовнішньої команди для перекладу повідомлень, щоб мати можливість використовувати різні бекенди: `aichat`, `ollama` напряму, mock-скрипт для тестування, або будь-яку іншу програму.

## Мета

З'ясувати оптимальний спосіб абстрагувати виклик зовнішньої команди для AI-перекладу у проєкті `po-tools-rust`, щоб:
1. Можна було легко замінити `aichat` на `ollama`, `llm`, `mods` чи будь-який інший CLI-інструмент.
2. Можна було легко підставити mock для unit-тестів без створення shell-скриптів.
3. Інтерфейс залишався простим і не переускладнював архітектуру.

## Поточний стан

### Як зараз працює виклик AI

Наразі у проєкті є функція `pipe_to_command()` у `src/util.rs`, яка:
- Приймає `command: &str`, `args: &[&str]`, `text: &str`
- Запускає зовнішній процес через `std::process::Command`
- Передає `text` через stdin
- Повертає stdout як `String`

Ця функція використовується у трьох AI-командах:

| Команда | Файл | Як використовує |
|---------|------|-----------------|
| `translate` | `command_translate_and_print.rs` | `pipe_to_command(config.aichat_command, config.aichat_options, &message_text)` |
| `review` | `command_review_files_and_print.rs` | `pipe_to_command(aichat_command, aichat_options, &message_text)` |
| `filter` | `command_filter_with_ai_and_print.rs` | `pipe_to_command(aichat_command, aichat_options, &message_text)` |

### Проблеми поточного підходу

1. **Жорстка прив'язка до `aichat`**: Команда та аргументи захардкоджені (`aichat -r role -m model`).
2. **Різні інструменти мають різний формат аргументів**:
   - `aichat -r translate-po -m ollama:gemma3 "prompt"` — через stdin
   - `ollama run gemma3 "prompt"` — prompt як останній аргумент, або через stdin
   - `llm -m gemma3 "prompt"` — через stdin або аргумент  
   - `mods -m ollama:gemma3 "prompt"` — через stdin
3. **Тести потребують створення shell-скриптів**: Зараз для тестів створюються тимчасові bash-скрипти, що працює, але є громіздким.

## Опис варіантів

### Варіант A: Загальна CLI-команда (рядок шаблону)

**Ідея**: Користувач задає повну команду як шаблон з placeholder'ом `{}` або через stdin:

```
po-tools translate --ai-command "ollama run gemma3" file.po
po-tools translate --ai-command "aichat -r translate-po -m ollama:gemma3" file.po
po-tools translate --ai-command "cat" file.po  # mock: echo input back
```

**Реалізація**: Мінімальні зміни. Замість окремих `--model`, `--role` параметрів — один параметр `--ai-command`, який розбивається на команду та аргументи. Текст промпту передається через stdin (як зараз).

```rust
// Новий Варіант: замість aichat_command + aichat_options — один рядок
struct AiBackend {
    command: String,
    args: Vec<String>,
}

impl AiBackend {
    fn from_command_line(cmd: &str) -> Self {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        AiBackend {
            command: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
        }
    }

    fn execute(&self, prompt: &str) -> Result<String> {
        pipe_to_command(&self.command, &self.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), prompt)
    }
}
```

**Зворотна сумісність**: Параметри `--model`, `--role`, `--rag` залишаються як скорочення для `aichat` (за замовчуванням). Якщо вказано `--ai-command`, ці параметри _ігноруються_ або генерують попередження.

**Переваги**:
- ✅ Дуже простий у реалізації
- ✅ Максимальна гнучкість — підтримує будь-яку команду
- ✅ Не потребує нового коду для кожного нового бекенду
- ✅ Легко mock-ати для тестів: `--ai-command "echo translated"`

**Недоліки**:
- ❌ Не дозволяє передати текст як аргумент (тільки stdin), але це нормально для більшості CLI
- ❌ Не валідує, чи команда правильна — помилка виявляється тільки при запуску

### Варіант B: Іменовані бекенди з конфігурацією

**Ідея**: Додати кілька вбудованих бекендів (`aichat`, `ollama`, `mock`) з відповідним форматуванням аргументів:

```
po-tools translate --backend aichat --model gemma3 file.po
po-tools translate --backend ollama --model gemma3 file.po
po-tools translate --backend mock file.po
```

**Реалізація**: Enum або trait з кількома реалізаціями:

```rust
enum AiBackend {
    Aichat { model: String, role: String, rag: Option<String> },
    Ollama { model: String },
    Mock { response: String },
    Custom { command: String, args: Vec<String> },
}

impl AiBackend {
    fn execute(&self, prompt: &str) -> Result<String> {
        match self {
            AiBackend::Aichat { model, role, rag } => {
                let mut args = vec!["-r", role, "-m", model];
                if let Some(rag) = rag {
                    args.extend(&["--rag", rag]);
                }
                pipe_to_command("aichat", &args, prompt)
            }
            AiBackend::Ollama { model } => {
                pipe_to_command("ollama", &["run", model], prompt)
            }
            AiBackend::Mock { response } => {
                Ok(response.clone())
            }
            AiBackend::Custom { command, args } => {
                pipe_to_command(command, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), prompt)
            }
        }
    }
}
```

**Переваги**:
- ✅ Зручний інтерфейс для відомих бекендів
- ✅ Вбудований mock — тести стають тривіальними
- ✅ Кожен бекенд може мати свою логіку форматування аргументів
- ✅ `Custom` варіант дає повну гнучкість

**Недоліки**:
- ❌ Більше коду для підтримки
- ❌ Потрібно додавати новий варіант для кожного нового інструменту
- ❌ Складніший парсинг аргументів командного рядка

### Варіант C: Trait-об'єкт (Strategy Pattern)

**Ідея**: Визначити trait `AiTranslator`, який абстрагує виклик AI:

```rust
trait AiTranslator {
    fn translate(&self, prompt: &str) -> Result<String>;
}

struct AichatTranslator { model: String, role: String }
struct OllamaTranslator { model: String }
struct MockTranslator { response: String }

impl AiTranslator for AichatTranslator {
    fn translate(&self, prompt: &str) -> Result<String> {
        pipe_to_command("aichat", &["-r", &self.role, "-m", &self.model], prompt)
    }
}

impl AiTranslator for MockTranslator {
    fn translate(&self, _prompt: &str) -> Result<String> {
        Ok(self.response.clone())
    }
}
```

**Переваги**:
- ✅ Класичний Rust-підхід
- ✅ Ідеальна тестовність — `MockTranslator` без зовнішніх процесів
- ✅ Розширюваність — легко додати HTTP-бекенд для ollama API

**Недоліки**:
- ❌ Найбільше коду та складності
- ❌ Надмірна абстракція для проєкту, який просто пайпить текст в зовнішню команду
- ❌ Ускладнює парсинг CLI-аргументів — потрібен фабричний метод

## Рекомендації

**Рекомендований варіант: A + елементи B (гібридний підхід).**

Обґрунтування:

1. **Основний механізм**: Структура `AiBackend` з полями `command: String` та `args: Vec<String>`, яка має метод `execute(prompt) -> Result<String>`.

2. **Зворотна сумісність**: Параметри `--model`, `--role`, `--rag` залишаються і формують аргументи для `aichat` (бекенд за замовчуванням).

3. **Новий параметр**: `--ai-command "ollama run gemma3"` — дозволяє вказати будь-яку команду. Якщо вказано, то `--model`/`--role`/`--rag` ігноруються.

4. **Для тестів**: Додати `AiBackend::mock(response: &str)` — повертає захардкоджену відповідь без запуску зовнішнього процесу. Це усуває потребу у створенні тимчасових shell-скриптів.

5. **Спільний для всіх AI-команд**: `AiBackend` використовується однаково у `translate`, `review`, та `filter`.

### Рекомендований дизайн

```rust
/// Бекенд для виклику AI-моделі.
pub struct AiBackend {
    command: String,
    args: Vec<String>,
    mock_response: Option<String>, // Для тестів
}

impl AiBackend {
    /// Створити бекенд для aichat (за замовчуванням).
    pub fn aichat(model: &str, role: &str, rag: Option<&str>) -> Self {
        let mut args = vec!["-r".into(), role.into(), "-m".into(), model.into()];
        if let Some(rag) = rag {
            args.push("--rag".into());
            args.push(rag.into());
        }
        Self { command: "aichat".into(), args, mock_response: None }
    }

    /// Створити бекенд з довільної команди.
    pub fn from_command_string(cmd: &str) -> Self {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        Self {
            command: parts[0].into(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            mock_response: None,
        }
    }

    /// Створити mock-бекенд для тестів.
    #[cfg(test)]
    pub fn mock(response: &str) -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            mock_response: Some(response.into()),
        }
    }

    /// Виконати запит до AI-моделі.
    pub fn execute(&self, prompt: &str) -> Result<String> {
        if let Some(ref response) = self.mock_response {
            return Ok(response.clone());
        }
        pipe_to_command(&self.command, &self.args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), prompt)
    }
}
```

### Зміна CLI-інтерфейсу

Додається **один** новий параметр для AI-команд:

```
po-tools translate --ai-command "ollama run gemma3" file.po
po-tools translate --ai-command "cat" file.po               # echo back (debug)
po-tools translate --ai-command "./my-mock.sh" file.po       # custom mock
po-tools translate -m ollama:gemma3 file.po                  # aichat (за замовчуванням)
```

## Вплив на проєкт

### Файли, що зміняться:

| Файл | Зміни |
|------|-------|
| `src/util.rs` | Додати `AiBackend` struct та методи |
| `src/command_translate_and_print.rs` | Замінити `aichat_command`/`aichat_options` на `AiBackend`; додати парсинг `--ai-command`; спростити тести |
| `src/command_review_files_and_print.rs` | Аналогічні зміни; додати парсинг `--ai-command`; спростити тести |
| `src/command_filter_with_ai_and_print.rs` | Аналогічні зміни; додати парсинг `--ai-command`; спростити тести |
| `TODO.md` | Додати завершене завдання |
| `Специфікація.md` | Додати опис `--ai-command` параметру |

### Файли, що не зміняться:

Усі інші команди (`sort`, `merge`, `diff`, тощо) не використовують AI і не зачіпаються.

### Обсяг змін

- **Новий код**: ~50-70 рядків (`AiBackend` + конструктори + метод `execute`)
- **Зміни у кожній AI-команді**: ~10-15 рядків (парсинг `--ai-command`, створення `AiBackend`, заміна виклику)
- **Спрощення тестів**: Видалення коду створення тимчасових shell-скриптів (~20 рядків на файл)
- **Загальний обсяг**: ~150 рядків змін
