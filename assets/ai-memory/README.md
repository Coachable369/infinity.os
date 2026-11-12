# Potion Base 2M

The packaged weights come from MinishLab’s MIT-licensed pretrained model. The source revision is [389b9f64](https://huggingface.co/minishlab/potion-base-2M/tree/389b9f64be5aa4ae7a6bc6fe95ef20ce485ae5da).

The source model.safetensors SHA-256 is f95ffde02ad06f63ae38eb9d400038cd5ccaf8411ec3cb650c6025113f96cbb8. The tensor contains 29,528 rows of 64 little-endian float32 values. The tokenizer.json WordPiece vocabulary is sorted by UTF-8 bytes. tokens.bin concatenates those bytes. offsets.bin contains one little-endian u32 per entry and a final end offset. weights.bin concatenates each token’s original row in that same sorted order. The conversion performs no quantization or retraining. See LICENSE.
