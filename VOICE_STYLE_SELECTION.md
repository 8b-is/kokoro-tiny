# Voice Style Selection Implementation

This document explains the voice style selection feature implemented to address the issue of clipped words and quiet output.

## Problem

The original implementation only used the first style variation (index 0) of each voice. Voice data contains 510 style variations per voice (stored with shape 510, 1, 256), where each style is optimized for different token counts. Using only the first style resulted in:
- Clipped words
- Inconsistent volume
- Poor quality for certain text lengths

## Solution

### 1. Load All Style Variations

Modified `load_voices()` function to load all 510 style variations for each voice:

```rust
// Before: Only loaded style index 0
let data = arr.as_slice()[0..256].to_vec();
voices.insert(name, data);

// After: Load all styles
let mut styles = Vec::new();
for i in 0..num_styles {
    let start = i * style_size;
    let end = start + style_size;
    let style_data = arr_slice[start..end].to_vec();
    styles.push(style_data);
}
voices.insert(name, styles);
```

### 2. Select Style Based on Token Count

The `synthesize()` method now selects the appropriate style based on the number of tokens:

```rust
let token_count = tokens[0].len();
let style_index = token_count.min(voice_styles.len() - 1);
let style = voice_styles[style_index].clone();
```

This ensures:
- Short text (few tokens) uses styles optimized for short sequences
- Long text (many tokens) uses styles optimized for longer sequences
- Natural progression as text length increases

### 3. Handle Token Limit (512 tokens)

Added automatic text splitting when input exceeds the 512 token limit:

```rust
if token_count > MAX_TOKENS {
    return self.synthesize_with_token_limit(text, voice, speed);
}
```

The splitting strategy:
1. **Primary**: Split by sentences (`.!?`)
2. **Fallback**: Split by words if sentences are still too long
3. **Merging**: Combine sentences into chunks up to the token limit
4. **Pauses**: Add natural pauses between chunks

### 4. Internal Implementation Details

- `synthesize()`: Public API that checks token limits
- `synthesize_direct()`: Internal method that doesn't check limits (prevents recursion)
- `synthesize_with_token_limit()`: Splits text by sentences
- `synthesize_by_words()`: Fallback word-level splitting

## Usage

The changes are transparent to existing code:

```rust
// Automatically selects appropriate style
let audio = tts.synthesize("Hello world!", None, None)?;

// Automatically splits if exceeding token limit
let audio = tts.synthesize(very_long_text, None, None)?;
```

## Testing

Run the test example to see the feature in action:

```bash
cargo run --example test_voice_styles
```

This will generate audio files demonstrating:
- Short text with appropriate style
- Medium text with appropriate style
- Long text with appropriate style
- Very long text with automatic chunking

## Benefits

1. **Better Audio Quality**: Styles optimized for token count produce clearer speech
2. **No Token Limit Errors**: Automatic splitting prevents errors
3. **Backward Compatible**: Existing code works without changes
4. **Natural Pauses**: Chunked text includes appropriate pauses
