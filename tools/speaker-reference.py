#!/usr/bin/env python3
"""Reference-only host verification; never packaged into InfinityOS."""
import sys
import wave
from pathlib import Path
import numpy as np
import onnxruntime as ort
import kaldi_native_fbank as knf

# ------------------------=
# FUNC: main
# DESC: Produces Kaldi and ONNX numerical references from recorded speech without accepting prose as test evidence.
# ------------------=
def main():
    folder=Path(sys.argv[1])
    folder.mkdir(parents=True,exist_ok=True)
    with wave.open(sys.argv[2]) as f:
        assert f.getframerate()==16000 and f.getsampwidth()==2 and f.getnchannels()==1
        pcm=np.frombuffer(f.readframes(f.getnframes()),dtype='<i2')
    # Fixed two-second crop; the guest performs speech admission before invoking this frontend.
    pcm=pcm[:32240]
    assert len(pcm)==32240
    pcm.tofile(folder/'pcm.bin')
    options=knf.FbankOptions()
    options.frame_opts.dither=0
    options.frame_opts.snip_edges=True
    options.mel_opts.num_bins=80
    fbank=knf.OnlineFbank(options)
    fbank.accept_waveform(16000,pcm.astype(np.float32).tolist())
    fbank.input_finished()
    features=np.stack([fbank.get_frame(i) for i in range(fbank.num_frames_ready)])
    assert features.shape==(200,80)
    features-=features.mean(axis=0)
    features.T.copy().tofile(folder/'reference-features.bin')
    sess=ort.InferenceSession('model-cache/speaker/resnet34.onnx',providers=['CPUExecutionProvider'])
    embedding=sess.run(None,{sess.get_inputs()[0].name:features[None]})[0].ravel()
    embedding/=np.linalg.norm(embedding)
    embedding.tofile(folder/'reference-embedding.bin')
    if (folder/'native-embedding.bin').exists():
        native=np.fromfile(folder/'native-embedding.bin',dtype=np.float32)
        nf=np.fromfile(folder/'native-features.bin',dtype=np.float32).reshape(80,200)
        error=np.max(np.abs(nf-features.T))
        cosine=np.dot(native,embedding)
        print({'filterbank_max_error':float(error),'embedding_cosine':float(cosine)})
        assert error<0.02
        assert cosine>0.999

if __name__=='__main__':
    main()
