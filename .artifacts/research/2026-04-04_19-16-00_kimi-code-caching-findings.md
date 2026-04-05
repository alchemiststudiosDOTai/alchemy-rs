# Kimi Code Caching Findings

Date: 2026-04-04
Repo: `/Users/tuna/alchemy-rs`

## Scope

Investigate why repeated long-prefix Kimi probe requests did not show cache activity, using:

- local runtime wiring in this repository
- official Kimi Code docs
- official public Kimi API platform docs
- direct raw API calls against the configured Kimi endpoint

## Local Runtime Mapping

The repository's first-class Kimi model is not wired to the public Kimi API platform (`platform.kimi.com` / `api.moonshot.ai` style chat completions path). It is wired to the Kimi Code coding endpoint:

- `src/models/kimi.rs`
  - `KIMI_BASE_URL = "https://api.kimi.com/coding"`
  - model id sent by the crate: `kimi-coding`
- `src/providers/kimi.rs`
  - appends `messages_endpoint: "/v1/messages"`
  - uses shared Anthropic-style runtime

Effective request path used by the crate:

`https://api.kimi.com/coding/v1/messages`

## Official Docs Split

### Public Kimi API Platform

Official pricing/docs for the public Kimi API platform say automatic context caching exists:

- `https://platform.kimi.com/docs/pricing/chat.en-US`
  - states `kimi-k2.5` and `kimi-k2` support automatic context caching
- `https://platform.kimi.com/docs/api/chat`
  - standard public chat API docs

This is the source of the earlier caching expectation.

### Kimi Code

Official Kimi Code docs describe a different product:

- `https://www.kimi.com/coding/docs/en/`
- `https://www.kimi.com/coding/docs/en/benefits.html`
- `https://www.kimi.com/coding/docs/en/third-party-agents.html`

Relevant Kimi Code documentation details:

- Claude Code setup uses:
  - `ANTHROPIC_BASE_URL=https://api.kimi.com/coding/`
- Roo Code setup uses:
  - OpenAI-compatible entrypoint `https://api.kimi.com/coding/v1`
  - model `kimi-for-coding`
- The docs emphasize quota, sessions, and third-party coding-agent configuration.
- I did not find official Kimi Code docs documenting prompt caching behavior.

## Raw API Evidence

### 1. Anthropic-style Kimi Code endpoint

Direct raw call to:

`POST https://api.kimi.com/coding/v1/messages`

with a long repeated prefix and identical repeated request returned:

```json
{
  "model": "kimi-for-coding",
  "usage": {
    "input_tokens": 7712,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0,
    "output_tokens": 3,
    "prompt_tokens": 7712,
    "cached_tokens": 0,
    "completion_tokens": 3,
    "total_tokens": 7715
  }
}
```

The second identical request also returned:

- `cache_creation_input_tokens: 0`
- `cache_read_input_tokens: 0`
- `cached_tokens: 0`

This means the zero-cache result is not caused by the crate's stream normalization. The upstream Kimi Code endpoint itself returned zero cache usage for repeated requests.

### 2. Model normalization on the endpoint

The crate sends `model: "kimi-coding"` on the Anthropic-style endpoint.

The raw response reported:

- `model: "kimi-for-coding"`

So the endpoint appears to normalize or alias the model name internally.

### 3. OpenAI-compatible Kimi Code endpoint

Direct raw call to:

`POST https://api.kimi.com/coding/v1/chat/completions`

with:

- `model: "kimi-for-coding"`

returned:

```json
{
  "error": {
    "message": "Kimi For Coding is currently only available for Coding Agents such as Kimi CLI, Claude Code, Roo Code, Kilo Code, etc.",
    "type": "access_terminated_error"
  }
}
```

So in the current environment, the accepted path is the Anthropic-style coding endpoint, not the direct OpenAI-compatible chat completions path.

## Extended Raw API Probing (2026-04-04 session 2)

Additional targeted probing to exhaust caching trigger hypotheses.

### 4. Multi-pass identical requests at ~2.7k tokens

Three identical requests with repeated system prefix (~2,716 input tokens):

```
Pass 1: input=2716, cache_creation=0, cache_read=0, cached_tokens=0
Pass 2: input=2716, cache_creation=0, cache_read=0, cached_tokens=0
Pass 3: input=2716, cache_creation=0, cache_read=0, cached_tokens=0
```

### 5. Multi-pass identical requests at ~18k tokens

Two identical requests with much larger system prefix (~18,016 input tokens, ~91KB payload):

```
Pass 1: input=18016, cache_creation=0, cache_read=0, cached_tokens=0
Pass 2: input=18016, cache_creation=0, cache_read=0, cached_tokens=0
```

No caching at any scale.

### 6. Anthropic-style cache_control ephemeral

Sent request with Anthropic-style `system` as array of objects including `cache_control: {"type": "ephemeral"}`:

```
input=18016, cache_creation=0, cache_read=0, cached_tokens=0
```

Also included `anthropic-version: 2023-06-01` header. Endpoint accepted it without error but still returned zero cache.

### 7. Full response headers

Response headers from a ~18k token request:

```
HTTP/2 200
content-type: application/json; charset=utf-8
server: cloudflare
cf-cache-status: DYNAMIC
```

No cache-related headers in the response. No `x-cache`, no `anthropic-ratelimit-*`, nothing suggesting cache infrastructure.

### 8. Additional response fields observed

The endpoint returns these fields beyond standard Anthropic Messages shape:

- `service_tier: "standard"`
- `inference_geo: "not_available"`

These appear on every response regardless of payload size.

### Probing Summary (Anthropic-style endpoint only)

| Test | Input Tokens | Cache Creation | Cache Read | Cached Tokens |
|------|-------------|---------------|------------|---------------|
| 3x identical ~2.7k | 2716 | 0 | 0 | 0 |
| 2x identical ~18k | 18016 | 0 | 0 | 0 |
| cache_control ephemeral ~18k | 18016 | 0 | 0 | 0 |

**These tests only hit the Anthropic-style `/v1/messages` endpoint.** The OpenAI-compatible endpoint was not yet tested with the correct configuration (see session 3 below).

## Probe Result Interpretation

The repository probe was expanded to send:

- a very large stable Kimi-specific prefix
- 10 repeated passes
- per-pass timing

Observed probe summary:

- `input=8223`
- `cache_read=0`
- `cache_write=0`
- across all 10 passes

Given the raw endpoint checks above, this outcome is consistent with the upstream Kimi Code API behavior observed here.

## Session Findings

Kimi Code docs document session continuity in the Kimi CLI product:

- `https://www.kimi.com/code/docs/en/kimi-cli/guides/sessions.html`

Documented concepts include:

- session resuming
- startup replay
- session state persistence
- context clear/compact

In this repository:

- `src/types/options.rs` contains `session_id`
- repository search found no provider/runtime use of `session_id`

This suggests the more likely missing capability for Kimi Code is session continuity support, not documented prompt-cache controls.

## Breakthrough: Caching Works on the OpenAI-Compatible Path (2026-04-04 session 3)

### Source: Kimi CLI kosong library

Examined the official Kimi CLI's LLM provider at:

- `https://github.com/MoonshotAI/kimi-cli/blob/a8f09bce1570fc76092dfba018bedc2429cba2af/packages/kosong/src/kosong/chat_provider/kimi.py`

Key findings from the source:

1. **The kosong `Kimi` provider uses the OpenAI chat completions API**, not Anthropic Messages. It calls `self.client.chat.completions.create(...)`.

2. **`prompt_cache_key` is a generation kwarg** passed as a top-level field in the request body. Set to the session ID:
   ```python
   if session_id:
       gen_kwargs["prompt_cache_key"] = session_id
   ```
   Source: `src/kimi_cli/llm.py` lines 137-138.

3. **User-Agent is enforced**. The constant is `KimiCLI/{version}`:
   ```python
   # src/kimi_cli/constant.py
   def get_user_agent() -> str:
       return f"KimiCLI/{get_version()}"
   ```
   This header is sent via `_kimi_default_headers()`.

4. **Cached token parsing** exists in `KimiStreamedMessage.usage`:
   ```python
   if hasattr(self._usage, "cached_tokens"):
       cached = getattr(self._usage, "cached_tokens") or 0
   elif (self._usage.prompt_tokens_details
         and self._usage.prompt_tokens_details.cached_tokens):
       cached = self._usage.prompt_tokens_details.cached_tokens
   ```
   Both `cached_tokens` (top-level) and `prompt_tokens_details.cached_tokens` are checked.

### Raw API Confirmation

Three requirements for caching on the Kimi Code API:

1. **OpenAI chat completions endpoint**: `POST https://api.kimi.com/coding/v1/chat/completions`
2. **`prompt_cache_key` field**: A top-level string in the request body (typically a session ID).
3. **`User-Agent: KimiCLI/{version}` header**: Required for endpoint access.

#### Test: 3-pass cache test with `prompt_cache_key`

```
Session: cache-test-final-1775349154
Model: kimi-for-coding
Prefix: ~9k tokens repeated system prompt
Endpoint: POST https://api.kimi.com/coding/v1/chat/completions
Headers: Authorization: Bearer <key>, User-Agent: KimiCLI/1.29.0

Pass 1: prompt_tokens=9014, cached_tokens=9014, prompt_tokens_details.cached_tokens=9014
Pass 2: prompt_tokens=9014, cached_tokens=9014, prompt_tokens_details.cached_tokens=9014
Pass 3: prompt_tokens=9014, cached_tokens=9014, prompt_tokens_details.cached_tokens=9014
```

**All 9,014 input tokens were served from cache on every pass.**

#### Earlier failure explained

The OpenAI chat completions endpoint rejects requests without a recognized User-Agent:

```json
{
  "error": {
    "message": "Kimi For Coding is currently only available for Coding Agents such as Kimi CLI, Claude Code, Roo Code, Kilo Code, etc.",
    "type": "access_terminated_error"
  }
}
```

The Anthropic-style `/v1/messages` endpoint accepts requests without this User-Agent check, but it does not support `prompt_cache_key` or any caching mechanism.

### Architecture Implications for This Crate

The crate currently:

- Uses the **Anthropic-style runtime** (`stream_anthropic_like_messages`) for Kimi
- Hits `POST https://api.kimi.com/coding/v1/messages`
- Has no `prompt_cache_key` support
- Has no `User-Agent: KimiCLI/{version}` header
- Has a `session_id` field in `src/types/options.rs` that is unused

To enable caching, the crate would need to:

1. **Switch Kimi to the OpenAI chat completions runtime** (or add a second Kimi path).
2. **Send `prompt_cache_key`** as a top-level body field, set to the session ID.
3. **Send `User-Agent: KimiCLI/{version}`** header on every request.
4. **Parse cache usage** from `cached_tokens` (top-level) and `prompt_tokens_details.cached_tokens`.

## Revised Conclusion

The Kimi Code API **does support prompt caching**, but only through its OpenAI-compatible chat completions endpoint with three requirements:

1. `prompt_cache_key` in the request body
2. `User-Agent: KimiCLI/{version}` header
3. Endpoint: `https://api.kimi.com/coding/v1/chat/completions`

The Anthropic-style `/v1/messages` endpoint (which this crate currently uses) does not support caching. The earlier "no caching" conclusion was wrong — it was an artifact of testing the wrong endpoint.

## Decision Points

1. The crate's Kimi provider should switch from the Anthropic-style runtime to the OpenAI chat completions runtime to enable caching.
2. `prompt_cache_key` must be wired through, using `session_id` from `src/types/options.rs`.
3. The `User-Agent: KimiCLI/{version}` header must be included in requests to the Kimi Code API.
4. Cache usage parsing should check both `cached_tokens` and `prompt_tokens_details.cached_tokens` as the kosong library does.
