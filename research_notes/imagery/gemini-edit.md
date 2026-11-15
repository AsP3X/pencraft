# Gemini: what an edit looks like (live service, 2026-11-15)

Endpoint `POST https://generativelanguage.googleapis.com/v1beta/interactions`, header `x-goog-api-key`, key on a
billed project. Every request had
`"response_format": {"type":"image","mime_type":"image/jpeg","image_size":"1K"}` plus `"aspect_ratio"` where said.
The subject was a generated 1200×896 JPEG (a white mug on a wooden table); the logo a local 256×256 PNG (a red
circle on white). Picture data is left out below.

| Call | Model | `input` after the text | Status | Result |
|---|---|---|---|---|
| One picture, no aspect | lite | subject | 200, 6.4 s | 1200×896: the input's own shape. The same mug and table at night with a candle |
| One picture, `aspect_ratio` 1:1 | lite | subject | 200, 5.0 s | 1024×1024, content kept |
| Picture and reference | lite | subject, logo (PNG) | 200, 5.8 s | 1200×896: the same scene, the red circle on the mug |
| Picture and reference | full | subject, logo (PNG) | 200, 9.8 s | 1200×896, same |
| Six pictures | full | subject, five logos | 200, 12.6 s | 1200×896: accepted |
| A 4096×3072 JPEG | lite | the subject scaled up | 200, 6.8 s | 1195×896: accepted, result at the 1K size |
| A PNG as the one to change | lite | logo (PNG) | 200, 3.8 s | 1024×1024 |
| A PNG sent as `image/jpeg` | lite | logo, wrong kind | **400** | `{"error":{"message":"Image generation blocked due to copyright/recitation. Please modify your input and retry.","code":"image_recitation"}}` |

"lite" is `gemini-3.1-flash-lite-image`, "full" `gemini-3.1-flash-image`.

The answer to an edit has the shape of a generation's: the picture is the `image` block (`mime_type`
`image/jpeg`, base64 `data`) in the `content` of the step of type `model_output`. A picture item in the
request is `{"type":"image","mime_type":"image/jpeg"|"image/png","data":"<base64>"}`.

## What this means for the build

1. **Leaving `aspect_ratio` out keeps the shape of the picture to change.** As the spec hoped; no fallback
   needed.
2. **The lite model takes a reference** and used it well, in spite of Google's "not optimized for multiple
   reference inputs". No special failure message is needed for references; the default model stays.
3. **Six pictures and a 4096-pixel picture are accepted.** Pencraft's own limits (one to change, four
   references, 2048 pixels) are well inside what the service takes; they stay as limits of cost and size.
4. **PNG input works**, so a picture with transparency can go as PNG.
5. **A refusal comes as status 400 with an `error.code` that starts with `image_`** (seen: `image_recitation`).
   Stage 1 reads only a 2xx answer without a picture as a refusal; `gemini::read` must also read a 400 whose
   code starts with `image_` as `Failure::Refused` with the service's message. (Why a wrong `mime_type` leads
   to this particular message is unclear; the kind is always right in Pencraft, since it is taken from the
   bytes.)
6. Results are JPEG at the size asked (`image_size`), not at the input's pixel size: a 4096-wide input comes
   back about 1200 wide at "1K".
