# [REST API](https://soniox.com/docs/stt/api-reference#rest-api)

REST API is available at [https://api.soniox.com/v1](https://api.soniox.com/v1/docs) and is divided into:

- **[Auth API](https://soniox.com/docs/stt/api-reference/auth/create_temporary_api_key)**: Create temporary API keys.
- **[Files API](https://soniox.com/docs/stt/api-reference/files/get_files)**: Manage audio files by uploading, listing, retrieving, and deleting them.
- **[Models API](https://soniox.com/docs/stt/api-reference/models/get_models)**: List available models.
- **[Transcriptions API](https://soniox.com/docs/stt/api-reference/transcriptions/get_transcriptions)**: Create and manage transcriptions for audio files uploaded via the Files API.

## Models API

```markdown
# Get models
URL: /stt/api-reference/models/get_models

Retrieves list of available models and their attributes.

***

title: Get models
description: Retrieves list of available models and their attributes.
full: true
\_openapi:
method: GET
route: /v1/models
toc: \[]
structuredData:
headings: \[]
contents:

* content: Retrieves list of available models and their attributes.

***

<APIPage document={"./openapi/public-api.json"} operations={[{"path":"/v1/models","method":"get"}]} webhooks={[]} hasHead={false} />
```

# WebSocket API

## [Overview](https://soniox.com/docs/stt/api-reference/websocket-api#overview)

The **Soniox WebSocket API** provides real-time **transcription and translation** of live audio with ultra-low latency. It supports advanced features like **speaker diarization, context customization,** and **manual finalization** — all over a persistent WebSocket connection. Ideal for live scenarios such as meetings, broadcasts, multilingual communication, and voice interfaces.

------

## [WebSocket endpoint](https://soniox.com/docs/stt/api-reference/websocket-api#websocket-endpoint)

Connect to the API using:

```
wss://stt-rt.soniox.com/transcribe-websocket
```

------

## [Configuration](https://soniox.com/docs/stt/api-reference/websocket-api#configuration)

Before streaming audio, configure the transcription session by sending a JSON message such as:

```
{
  "api_key": "<SONIOX_API_KEY|SONIOX_TEMPORARY_API_KEY>",
  "model": "stt-rt-preview",
  "audio_format": "auto",
  "language_hints": ["en", "es"],
  "context": {
    "general": [
      { "key": "domain", "value": "Healthcare" },
      { "key": "topic", "value": "Diabetes management consultation" },
      { "key": "doctor", "value": "Dr. Martha Smith" },
      { "key": "patient", "value": "Mr. David Miller" },
      { "key": "organization", "value": "St John's Hospital" }
    ],
    "text": "Mr. David Miller visited his healthcare provider last month for a routine follow-up related to diabetes care. The clinician reviewed his recent test results, noted improved glucose levels, and adjusted his medication schedule accordingly. They also discussed meal planning strategies and scheduled the next check-up for early spring.",
    "terms": [
      "Celebrex",
      "Zyrtec",
      "Xanax",
      "Prilosec",
      "Amoxicillin Clavulanate Potassium"
    ],
    "translation_terms": [
      { "source": "Mr. Smith", "target": "Sr. Smith" },
      { "source": "St John's", "target": "St John's" },
      { "source": "stroke", "target": "ictus" }
    ]
  },
  "enable_speaker_diarization": true,
  "enable_language_identification": true,
  "translation": {
    "type": "two_way",
    "language_a": "en",
    "language_b": "es"
  }
}
```

------

### [Parameters](https://soniox.com/docs/stt/api-reference/websocket-api#parameters)

`api_key`Requiredstring

Your Soniox API key. Create API keys in the [Soniox Console](https://console.soniox.com/). For client apps, generate a [temporary API](https://soniox.com/docs/stt/api-reference/auth/create_temporary_api_key) key from your server to keep secrets secure.

`model`Requiredstring

Real-time model to use. See [models](https://soniox.com/docs/stt/models).

Example: `"stt-rt-preview"`

`audio_format`Requiredstring

Audio format of the stream. See [audio formats](https://soniox.com/docs/stt/rt/real-time-transcription#audio-formats).

`num_channels`number

Required for raw audio formats. See [audio formats](https://soniox.com/docs/stt/rt/real-time-transcription#audio-formats).

`sample_rate`number

Required for raw audio formats. See [audio formats](https://soniox.com/docs/stt/rt/real-time-transcription#audio-formats).

`language_hints`array<string>

See [language hints](https://soniox.com/docs/stt/concepts/language-hints).

`language_hints_strict`bool

See [language restrictions](https://soniox.com/docs/stt/concepts/language-restrictions).

`context`object

See [context](https://soniox.com/docs/stt/concepts/context).

`enable_speaker_diarization`boolean

See [speaker diarization](https://soniox.com/docs/stt/concepts/speaker-diarization).

`enable_language_identification`boolean

See [language identification](https://soniox.com/docs/stt/concepts/language-identification).

`enable_endpoint_detection`boolean

See [endpoint detection](https://soniox.com/docs/stt/rt/endpoint-detection).

`client_reference_id`string

Optional identifier to track this request (client-defined).

`translation`object

See [real-time translation](https://soniox.com/docs/stt/rt/real-time-translation).

**One-way translation**

`type`Requiredstring

Must be set to `one_way`.

`target_language`Requiredstring

Language to translate the transcript into.

**Two-way translation**

`type`Requiredstring

Must be set to `two_way`.

`language_a`Requiredstring

First language for two-way translation.

`language_b`Requiredstring

Second language for two-way translation.

------

## [Audio streaming](https://soniox.com/docs/stt/api-reference/websocket-api#audio-streaming)

After configuration, start streaming audio:

- Send audio as binary WebSocket frames.
- Each stream supports up to 300 minutes of audio.

------

## [Ending the stream](https://soniox.com/docs/stt/api-reference/websocket-api#ending-the-stream)

To gracefully close a streaming session:

- Send an **empty WebSocket frame** (binary or text).
- The server will return one or more responses, including [finished response](https://soniox.com/docs/stt/api-reference/websocket-api#finished-response), and then close the connection.

------

## [Response](https://soniox.com/docs/stt/api-reference/websocket-api#response)

Soniox returns **responses** in JSON format. A typical successful response looks like:

```
{
  "tokens": [
    {
      "text": "Hello",
      "start_ms": 600,
      "end_ms": 760,
      "confidence": 0.97,
      "is_final": true,
      "speaker": "1"
    }
  ],
  "final_audio_proc_ms": 760,
  "total_audio_proc_ms": 880
}
```

### [Field descriptions](https://soniox.com/docs/stt/api-reference/websocket-api#field-descriptions)

`tokens`array<object>

List of processed tokens (words or subwords).

Each token may include:

`text`string

Token text.

`start_ms`Optionalnumber

Start timestamp of the token (in milliseconds). Not included if `translation_status` is `translation`.

`end_ms`Optionalnumber

End timestamp of the token (in milliseconds). Not included if `translation_status` is `translation`.

`confidence`number

Confidence score (`0.0`–`1.0`).

`is_final`boolean

Whether the token is finalized.

`speaker`Optionalstring

Speaker label (if diarization enabled).

`translation_status`Optionalstring

See [real-time translation](https://soniox.com/docs/stt/rt/real-time-translation).

`language`Optionalstring

Language of the `token.text`.

`source_language`Optionalstring

See [real-time translation](https://soniox.com/docs/stt/rt/real-time-translation).

`final_audio_proc_ms`number

Audio processed into final tokens.

`total_audio_proc_ms`number

Audio processed into final + non-final tokens.

------

## [Finished response](https://soniox.com/docs/stt/api-reference/websocket-api#finished-response)

At the end of a stream, Soniox sends a **final message** to indicate the session is complete:

```
{
  "tokens": [],
  "final_audio_proc_ms": 1560,
  "total_audio_proc_ms": 1680,
  "finished": true
}
```

After this, the server closes the WebSocket connection.

------

## [Error response](https://soniox.com/docs/stt/api-reference/websocket-api#error-response)

If an error occurs, the server returns an **error message** and immediately closes the connection:

```
{
  "tokens": [],
  "error_code": 503,
  "error_message": "Cannot continue request (code N). Please restart the request. ..."
}
```

`error_code`number

Standard HTTP status code.

`error_message`string

A description of the error encountered.

Full list of possible error codes and messages:

### 400 Bad request

The request is malformed or contains invalid parameters.

- `Audio data channels must be specified for PCM formats`
- `Audio data sample rate must be specified for PCM formats`
- `Audio decode error`
- `Audio is too long.`
- `Client reference ID is too long (max length 256)`
- `Context is too long (max length 10000).`
- `Control request invalid type.`
- `Control request is malformed.`
- `Invalid audio data format: avi`
- `Invalid base64.`
- `Invalid language hint.`
- `Invalid model specified.`
- `Invalid translation target language.`
- `Language hints must be unique.`
- `Missing audio format. Specify a valid audio format (e.g. s16le, f32le, wav, ogg, flac...) or "auto" for auto format detection.`
- `Model does not support translations.`
- `No audio received.`
- `Prompt too long for model`
- `Received too much audio data in total.`
- `Start request is malformed.`
- `Start request must be a text message.`

### 401 Unauthorized

Authentication is missing or incorrect. Ensure a valid API key is provided before retrying.

- `Invalid API key.`
- `Invalid/expired temporary API key.`
- `Missing API key.`

### 402 Payment required

The organization's balance or monthly usage limit has been reached. Additional credits are required before making further requests.

- `Organization balance exhausted. Please either add funds manually or enable autopay.`
- `Organization monthly budget exhausted. Please increase it.`
- `Project monthly budget exhausted. Please increase it.`

### 408 Request timeout

The client did not send a start message or sufficient audio data within the required timeframe. The connection was closed due to inactivity.

- `Audio data decode timeout`
- `Input too slow`
- `Request timeout.`
- `Start request timeout`
- `Timed out while waiting for the first audio chunk`

### 429 Too many requests

A usage or rate limit has been exceeded. You may retry after a delay or request an increase in limits via the Soniox Console.

- `Rate limit for your organization has been exceeded.`
- `Rate limit for your project has been exceeded.`
- `Your organization has exceeded max number of concurrent requests.`
- `Your project has exceeded max number of concurrent requests.`

### 500 Internal server error

An unexpected server-side error occurred. The request may be retried.

- `The server had an error processing your request. Sorry about that! You can retry your request, or contact us through our support email support@soniox.com if you keep seeing this error.`

### 503 Service unavailable

Cannot continue request or accept new requests.

- `Cannot continue request (code N). Please restart the request. Refer to: https://soniox.com/url/cannot-continue-request`
