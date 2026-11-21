# Tokens and price of an answer

Date: 2026-11-20. Status: built; not committed; plan in `docs/superpowers/plans/2026-11-20-token-usage-and-price.md`.

## Goal

The chat says how many tokens an answer took and what it cost, for every engine and every provider Pencraft
can reach, by the price that provider charges for that model. Where a number isn't known the chat says less,
never a guess.

## Behaviour

- Under the last words of every answered message stands one quiet line: `12.4k in · 1.8k out · $0.08`.
  "In" is everything the model read (cached or not), "out" everything it wrote (thinking included), summed
  over all the requests the message took, the review's included.
- The line's places: the finished sum stands under the answer it belongs to. While the assistant works, a
  small pill in the gap between the conversation and the message box counts up, where it is seen without
  scrolling. With the sidebar folded away a small pill over the message bar shows the count, and afterwards
  what the last answer took, under the line that says what the assistant is doing.
- While the assistant works, that pill counts up: the tokens of
  the finished requests, plus what the request under way has taken so far where its engine says (the Messages
  API says what was read as an answer starts; Claude Code and Grok say it request by request). It shows no
  price while a request is under way. A request that never reports (it broke off, or the run was stopped)
  keeps what was counted, and the run's sum is then written `≥`. Chat Completions and Ollama say nothing
  until a request is done, so their line moves once per request.
- Hovering the line shows the parts: input, read from cache, written to cache, output, the model that
  answered, where the price comes from, and the total of the whole conversation.
- The price is written by what is known:
  - billed by the provider for exactly this run: `$0.08`
  - worked out from a price list: `$0.08`, and the hover names the list
  - a provider or program paid by subscription (OpenCode Go, Kimi For Coding, Ollama Cloud, Claude Code,
    Grok): `≈ $0.08`, and the hover says it is what the tokens are worth at list prices, not a bill
  - no price known: the tokens alone
  - Ollama on this computer: the tokens alone; the hover says the model runs on this computer
- A message whose requests were answered by models with and without a known price shows the known part as
  `≥ $0.08`.
- Less than a cent is `< $0.01`. Tokens are written as `940`, `12.4k`, `1.2M`.
- An engine that reports nothing (a program that only prints its answer) gets no line.
- A run that ends in a problem still gets its line, under the problem, for what it used until then.
- The conversation's total is kept with the design's chat and is there again when the design is opened.
  Clearing the chat starts it at nothing.
- The settings get one switch, **Look up model prices**, beside "Have finished designs reviewed", on by default, with the hint that the public
  price list at models.dev is fetched once a day and nothing about the user or their chats is sent. Off, only
  billed prices and the user's own are shown.
- The model form of a provider gets two optional fields beside the context window: **Input price** and
  **Output price**, in dollars per million tokens. A number there wins over the price list.

Known limits:

- An answer that breaks off and is asked again shows only the tokens of the request that succeeded.
- Usage a run reports after it was stopped is not counted.
- A run that ended in a problem keeps its tokens in the conversation's total, but its own line is gone when
  the design is reopened (problem lines are not kept).

Not included: pictures made by the image tools (priced per picture, not per token), a budget or a warning at
a sum, totals across designs, other currencies than the dollar, prices read from a provider's own model list
(the one provider checked to list them, OpenRouter, bills each request anyway).

## Where the numbers come from

### Tokens

| Engine | Read from | Change |
|---|---|---|
| Messages API (`chat/api.rs`) | `message_start`: `message.usage` and `message.model`; `message_delta`: `usage.output_tokens` (the last one counts) | read what already arrives |
| Chat Completions (`chat/openai.rs`) | the stream's last chunk: `usage.prompt_tokens`, `completion_tokens`, `prompt_tokens_details.cached_tokens` (Deepseek: `prompt_cache_hit_tokens`), `usage.cost` where the provider bills in the answer (OpenRouter) | the request gains `stream_options: { include_usage: true }` |
| Ollama (`chat/ollama.rs`) | the `done` line: `prompt_eval_count`, `eval_count` | read what already arrives |
| Claude Code, Grok (`chat/code.rs`) | the `result` line: `usage` and `total_cost_usd` | read what already arrives |
| Other programs (`chat/plain.rs`) | nothing | none |

All of them are brought to one shape: input not from cache, input read from cache, input written to cache,
output. Chat Completions counts cached tokens inside `prompt_tokens`, so they are taken off there; the Messages
API counts them apart already.

A request refused with 400 or 422 while it carries `stream_options` is sent once more without it, before the
existing try without pictures. If that goes through, the provider's address is remembered until the app
closes, so later requests don't ask twice; such a provider shows no line. If it is refused again, the reason
was another one and later requests keep asking.

### Prices

Best first; the first that knows the model decides.

1. **Billed.** The cost the provider or program reports for the request (`usage.cost`, `total_cost_usd`).
   Used as it is.
2. **The user's own.** The two price fields of the model. Cached input is charged as input.
3. **The price list.** `https://models.dev/api.json`, the open list the OpenCode project keeps. Checked on
   2026-11-20: 324 providers, 6.8 MB, prices in dollars per million tokens as `cost.input`, `cost.output`,
   `cost.cache_read`, `cost.cache_write`, and `cost.tiers` for models that cost more past a context size
   (the only tier kind is `context`).
4. **Unknown.** Tokens only.

A provider is found in the list by this table for the known ones, and for a provider the user added by its
address (the list's `api` field, compared without the slash at the end):

| Pencraft | models.dev |
|---|---|
| OpenAI ChatGPT | `openai` |
| Anthropic Claude Code | `anthropic` |
| xAI | `xai` |
| Google Gemini | `google` |
| OpenCode Zen | `opencode` |
| OpenCode Go | `opencode-go` |
| Open Router | `openrouter` |
| Ollama Cloud | `ollama-cloud` |
| Meta | `meta` |
| Moonshot AI | `moonshotai` |
| Kimi For Coding | `kimi-code-plan-global` |
| Deepseek | `deepseek` |
| Together AI | `togetherai` |
| Fireworks AI | `fireworks-ai` |
| Z.AI | `zai` |
| Ollama (this computer) | none: no price |

A model is found by its exact name. A model whose prices in the list are all zero has no price (the list
writes subscription plans and free models that way). With a tier, a request whose whole input is larger than
the tier's size is charged at the tier's prices. A missing `cache_read` or `cache_write` is charged as input.

Paid by subscription, so shown with `≈`, are: the known providers OpenCode Go, Kimi For Coding and Ollama
Cloud (by their address), and every answer of Claude Code and Grok, whose programs report a cost whether the
user pays per token or by plan and don't say which.

The Messages API may hand a request to another model (`fallbacks`); the request is priced by the model
`message_start` names.

## How

- `chat::usage` (new, `src/chat/usage.rs`), no window, no network:
  - `Used`: the four token counts of one request or a sum of them; adds up.
  - `Rates`: the four prices of a model and its tiers, and `cost(&Used)`.
  - `Spent`: `Used`, the cost if known, how it is known (`Billed`, `Listed`, `Own`, `Worth` for a
    subscription, `Local`, `Unknown`), and the model's name. Adds up: a sum of a known and an unknown cost
    keeps the known part and is marked as incomplete, which the line shows as `≥ $0.08`.
  - The words of the line and of the hover.
  - Money is kept in whole numbers: a price as millionths of a dollar per million tokens, a cost as tokens
    times that (so in millionths of a millionth of a dollar; a `u64` holds 18 million dollars). Sums are exact
    and `Entry` stays `Eq`; dollars appear only in the words.
- `chat::prices` (new, `src/chat/prices.rs`):
  - reads the price list from `~/.pencraft/prices.json`, a cut of models.dev down to every provider's address
    and its models' prices (so a provider added later is found without another fetch);
  - `Catalogue::models(address)`: the prices of that provider's models by name, from memory;
  - a refresh on a thread of its own when the app starts and the file is older than a day, the switch is on,
    and at least one provider with a key is connected. It uses the connection the chat already has (`ureq`);
    a failed fetch keeps the old file and is tried again at the next start. No new dependency.
- `Event::Used(Report)` (new), a report being the tokens, the answering model's name and the billed cost where
  known: each engine tells it once per request, or once per run for the programs. The engines get nothing new;
  a run is handed a `Pricing` when it starts (`Ask::pricing`, built by `chat::pricing` from the chosen
  assistant, the settings and the list), and the chat prices each report with it.
- `Chat` gains `spent: Spent` for the run under way and `total: Spent` for the conversation. At `Event::Done`
  the run's sum is put on the run's last line (`Entry::spent: Option<Spent>`).
- `chat/history.rs`: an assistant's line keeps `"spent"`, and the file keeps `"total"` beside `"said"`. A file
  without them reads as before.
- `prefs.rs`: `ProviderModel` gains `input_price` and `output_price` (as typed, dollars per million tokens,
  empty for none); `Agents` gains `prices: bool`.
- `conversation.rs` paints the line and its hover; its colour and size come from `theme.rs`.
- `settings.rs`: the switch and the two fields.

## What the checks showed (2026-11-20)

Checked against the real programs and the live price list:

- Claude Code: the `result` line has `usage` (input, output, cache read and cache creation tokens) and
  `total_cost_usd`. `usage` is per message. `total_cost_usd` and `modelUsage` are cumulative over a resumed
  session, and the chat resumes the session for every message, so the cost is read from
  `model_usage_since_last_result`, which is per message.
- Grok: reports the same `usage` and `total_cost_usd`, both per message, and it gets a line.
- The live models.dev list has all 15 providers of the table with prices (Kimi's plan is zeros, as expected).

Still unchecked, as each needs the user's keys:

- which of the Chat Completions providers refuse `stream_options` (the fallback above covers them);
- that OpenRouter sends `usage.cost` without being asked;
- Meta's model names at `api.llama.com`;
- whether xAI and Gemini report thinking tokens outside `completion_tokens` (handled by taking the larger of
  that and total less prompt);
- Moonshot's cached-token field.

Each is settled by one captured answer, which then becomes a test.

## Tests

- Each engine's stream, with a captured usage in it, tells `Event::Used` with the right four counts; cached
  tokens are not counted twice for Chat Completions.
- A stream without usage tells nothing, and the run goes on as before.
- A request refused for `stream_options` is sent again without it, once, and not for the next run.
- `Rates::cost`: plain, with cache prices, without them, over and under a tier's size.
- A cut of the real models.dev answer is read; every provider in the table is found in it; an all-zero model
  has no price; a custom provider is found by its address.
- The order of prices: billed before the user's own before the list.
- Sums: known and unknown costs, and a subscription's worth.
- The line's words for every kind of price and for small and large numbers.
- A chat file with and without `spent` and `total` reads; one written is read back the same.
- An ignored test that fetches the real list and finds the table's providers in it.
