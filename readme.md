# tokenmeter.rs

Считает токены текста в `o200k_base`, кодировке GPT-4o, GPT-4.1, моделей серии o и GPT-5, и делит начало текста на группы токенов, не разрывая графемные кластеры. Токены группы не выходят за её края, поэтому группу можно показать целиком и подписать числом токенов.

## Установка

Rust:

```bash
cargo add tokenmeter-core
```

JavaScript:

```bash
pnpm add @meritt/tokenmeter-wasm
```

Swift:

```swift
.package(url: "https://github.com/meritt/tokenmeter.rs", exact: "0.1.0")
```

## Использование

Rust:

```rust
let tokens = tokenmeter_core::count("Пользователи — это наше всё."); // 9
let split = tokenmeter_core::split(text, 200);                       // Split { tokens, groups: [Group { end, tokens }, …], truncated }
```

JavaScript:

```js
import { load } from '@meritt/tokenmeter-wasm';

const meter = await load();
meter.count('Пользователи — это наше всё.');       // 9
meter.split('Пользователи — это наше всё.', 200);
// { tokens: 9, groups: [{ text: 'П', tokens: 1 }, { text: 'ольз', tokens: 1 }, …], truncated: false }
```

Swift:

```swift
import TokenMeter

TokenMeter.count("Пользователи — это наше всё.")   // 9
let split = try TokenMeter.split(text, limit: 200)  // split.groups: [Group(text:tokens:)]
```

`split` возвращает не больше `limit` групп, по умолчанию 200, и `truncated`, если текст продолжается дальше. Специальные токены вроде `<|endoftext|>` считаются как обычный текст. Таблицу кодировки `load` в JavaScript готовит сразу, а Rust и Swift — при первом подсчёте или заранее через `prepare()`.

Ошибки — `TokenMeterError` с кодом `invalid_argument` для неверного `limit`; в JavaScript ещё `invalid_utf8` для строки с одиночным суррогатом.

## Автор

[Алексей Симоненко](https://simonenko.xyz)

## Лицензия

MIT. Смотрите файл `LICENSE`.
