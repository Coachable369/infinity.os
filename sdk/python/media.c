// Native media decoding lives in the isolated CPython application, never in the kernel.
#define PY_SSIZE_T_CLEAN
#include "Python.h"
#include <infinity/syscall.h>
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/opt.h>
#include <libswresample/swresample.h>
#include <libswscale/swscale.h>
#include <stdint.h>
#include <string.h>

#define MEDIA_LIMIT (4 * 1024 * 1024)
#define CAP "InfinityOS.media.stream"
typedef struct {
    PyObject *source;
    const uint8_t *data;
    int size, pos, stream, eof, rate;
    AVFormatContext *format;
    AVIOContext *io;
    AVCodecContext *codec;
    AVPacket *packet;
    AVFrame *frame;
    SwrContext *resample;
    struct SwsContext *scale;
} Stream;

// ------------------------=
// FUNC: memory_read
// DESC: Reads only the authorized immutable in-process snapshot.
// ------------------=
static int memory_read(void *opaque, uint8_t *dst, int amount) {
    Stream *s = opaque;
    if (amount > s->size - s->pos)
        amount = s->size - s->pos;
    if (amount <= 0)
        return AVERROR_EOF;
    memcpy(dst, s->data + s->pos, amount);
    s->pos += amount;
    return amount;
}
// ------------------------=
// FUNC: memory_seek
// DESC: Validates decoder seeks inside the snapshot, including file-size queries.
// ------------------=
static int64_t memory_seek(void *opaque, int64_t offset, int whence) {
    Stream *s = opaque;
    whence &= ~AVSEEK_FORCE;
    if (whence == AVSEEK_SIZE)
        return s->size;
    int64_t base = whence == SEEK_SET   ? 0
                   : whence == SEEK_CUR ? s->pos
                   : whence == SEEK_END ? s->size
                                        : -1;
    if (base < 0 || offset < -base || offset > s->size - base)
        return AVERROR(EINVAL);
    return s->pos = (int)(base + offset);
}
// ------------------------=
// FUNC: release_stream
// DESC: Releases decoder, demuxer, custom IO, and the immutable Python snapshot.
// ------------------=
static void release_stream(Stream *s) {
    if (!s)
        return;
    sws_freeContext(s->scale);
    swr_free(&s->resample);
    av_frame_free(&s->frame);
    av_packet_free(&s->packet);
    avcodec_free_context(&s->codec);
    avformat_close_input(&s->format);
    if (s->io) {
        av_freep(&s->io->buffer);
        avio_context_free(&s->io);
    }
    Py_XDECREF(s->source);
    av_free(s);
}
// ------------------------=
// FUNC: close_stream
// DESC: Binds decoder cleanup to Python capsule ownership.
// ------------------=
static void close_stream(PyObject *capsule) { release_stream(PyCapsule_GetPointer(capsule, CAP)); }
// ------------------------=
// FUNC: open_stream
// DESC: Opens a single supported audio or video track from local bytes with bounded codec geometry.
// ------------------=
static PyObject *open_stream(PyObject *self, PyObject *args) {
    PyObject *raw;
    int video, rate;
    if (!PyArg_ParseTuple(args, "O!ii", &PyBytes_Type, &raw, &video, &rate))
        return NULL;
    if (PyBytes_GET_SIZE(raw) > MEDIA_LIMIT || rate < 8000 || rate > 48000) {
        PyErr_SetString(PyExc_ValueError, "Unsupported media size or output rate");
        return NULL;
    }
    Stream *s = av_mallocz(sizeof(*s));
    if (!s)
        return PyErr_NoMemory();
    s->source = Py_NewRef(raw);
    s->data = (const uint8_t *)PyBytes_AS_STRING(raw);
    s->size = PyBytes_GET_SIZE(raw);
    s->rate = rate;
    uint8_t *buffer = av_malloc(4096);
    if (!buffer ||
        !(s->io = avio_alloc_context(buffer, 4096, 0, s, memory_read, NULL, memory_seek))) {
        av_free(buffer);
        release_stream(s);
        return PyErr_NoMemory();
    }
    if (!(s->format = avformat_alloc_context())) {
        release_stream(s);
        return PyErr_NoMemory();
    }
    s->format->pb = s->io;
    s->format->flags |= AVFMT_FLAG_CUSTOM_IO;
    s->format->max_streams = 16;
    if (avformat_open_input(&s->format, NULL, NULL, NULL) < 0 ||
        avformat_find_stream_info(s->format, NULL) < 0)
        goto invalid;
    const AVCodec *decoder = NULL;
    s->stream = av_find_best_stream(s->format, video ? AVMEDIA_TYPE_VIDEO : AVMEDIA_TYPE_AUDIO, -1,
                                    -1, &decoder, 0);
    if (s->stream == AVERROR_STREAM_NOT_FOUND) {
        release_stream(s);
        Py_RETURN_NONE;
    }
    if (s->stream < 0 || !decoder || !(s->codec = avcodec_alloc_context3(decoder)))
        goto invalid;
    if (avcodec_parameters_to_context(s->codec, s->format->streams[s->stream]->codecpar) < 0)
        goto invalid;
    s->codec->thread_count = 1;
    s->codec->max_pixels = 4096 * 2160;
    if (avcodec_open2(s->codec, decoder, NULL) < 0 || !(s->packet = av_packet_alloc()) ||
        !(s->frame = av_frame_alloc()))
        goto invalid;
    PyObject *cap = PyCapsule_New(s, CAP, close_stream);
    if (!cap) {
        release_stream(s);
        return NULL;
    }
    return Py_BuildValue(
        "Nd", cap, s->format->duration > 0 ? (double)s->format->duration / AV_TIME_BASE : 0.0);
invalid:
    release_stream(s);
    PyErr_SetString(PyExc_ValueError, "Damaged file or unsupported codec");
    return NULL;
}
// ------------------------=
// FUNC: frame_bytes
// DESC: Converts decoded frames into hardware-rate stereo PCM or aspect-fit Tk PPM video frames.
// ------------------=
static PyObject *frame_bytes(Stream *s) {
    AVFrame *f = s->frame;
    if (s->codec->codec_type == AVMEDIA_TYPE_AUDIO) {
        if (f->sample_rate < 4000 || f->sample_rate > 384000 || f->nb_samples <= 0 ||
            f->nb_samples > 65536)
            return NULL;
        if (!s->resample) {
            AVChannelLayout stereo = AV_CHANNEL_LAYOUT_STEREO;
            if (swr_alloc_set_opts2(&s->resample, &stereo, AV_SAMPLE_FMT_S16, s->rate,
                                    &f->ch_layout, f->format, f->sample_rate, 0, NULL) < 0 ||
                swr_init(s->resample) < 0)
                return NULL;
        }
        int limit = swr_get_out_samples(s->resample, f->nb_samples);
        if (limit < 0 || limit > 160000)
            return NULL;
        PyObject *bytes = PyBytes_FromStringAndSize(NULL, (Py_ssize_t)limit * 4);
        if (!bytes)
            return NULL;
        uint8_t *out = (uint8_t *)PyBytes_AS_STRING(bytes);
        int n = swr_convert(s->resample, &out, limit, (const uint8_t **)f->extended_data,
                            f->nb_samples);
        if (n < 0) {
            Py_DECREF(bytes);
            return NULL;
        }
        if (_PyBytes_Resize(&bytes, (Py_ssize_t)n * 4) < 0)
            return NULL;
        return bytes;
    }
    if (f->width <= 0 || f->height <= 0)
        return NULL;
    int w = f->width, h = f->height;
    if (w > 512) {
        h = (int)((int64_t)h * 512 / w);
        w = 512;
    }
    if (h > 288) {
        w = (int)((int64_t)w * 288 / h);
        h = 288;
    }
    if (w < 1 || h < 1)
        return NULL;
    s->scale = sws_getCachedContext(s->scale, f->width, f->height, f->format, w, h,
                                    AV_PIX_FMT_RGB24, SWS_BILINEAR, NULL, NULL, NULL);
    if (!s->scale)
        return NULL;
    char header[64];
    int hn = snprintf(header, sizeof(header), "P6\n%d %d\n255\n", w, h);
    PyObject *bytes = PyBytes_FromStringAndSize(NULL, hn + w * h * 3);
    if (!bytes)
        return NULL;
    memcpy(PyBytes_AS_STRING(bytes), header, hn);
    uint8_t *out[4] = {(uint8_t *)PyBytes_AS_STRING(bytes) + hn};
    int pitch[4] = {w * 3};
    if (sws_scale(s->scale, (const uint8_t *const *)f->data, f->linesize, 0, f->height, out,
                  pitch) != h) {
        Py_DECREF(bytes);
        return NULL;
    }
    return bytes;
}
// ------------------------=
// FUNC: next_frame
// DESC: Drains queued decoded frames, feeding packets from the selected stream until EOF or error.
// ------------------=
static PyObject *next_frame(PyObject *self, PyObject *capsule) {
    Stream *s = PyCapsule_GetPointer(capsule, CAP);
    if (!s)
        return NULL;
    for (;;) {
        int rc = avcodec_receive_frame(s->codec, s->frame);
        if (rc == 0) {
            int64_t pts = s->frame->best_effort_timestamp;
            AVStream *st = s->format->streams[s->stream];
            double t = pts == AV_NOPTS_VALUE
                           ? 0.0
                           : (pts - (st->start_time == AV_NOPTS_VALUE ? 0 : st->start_time)) *
                                 av_q2d(st->time_base);
            PyObject *bytes = frame_bytes(s);
            av_frame_unref(s->frame);
            if (!bytes)
                break;
            return Py_BuildValue("dN", t, bytes);
        }
        if (rc == AVERROR_EOF)
            Py_RETURN_NONE;
        if (rc != AVERROR(EAGAIN) || s->eof)
            break;
        do {
            av_packet_unref(s->packet);
            rc = av_read_frame(s->format, s->packet);
        } while (rc == 0 && s->packet->stream_index != s->stream);
        if (rc == AVERROR_EOF) {
            s->eof = 1;
            rc = avcodec_send_packet(s->codec, NULL);
        } else if (rc >= 0)
            rc = avcodec_send_packet(s->codec, s->packet);
        if (rc < 0)
            break;
    }
    if (!PyErr_Occurred())
        PyErr_SetString(PyExc_ValueError, "Cannot decode this media frame");
    return NULL;
}
// ------------------------=
// FUNC: media_io
// DESC: Marshals only the player-specific bounded read, enumerate, audio and transport operations.
// ------------------=
static PyObject *media_io(PyObject *self, PyObject *args) {
    int op;
    Py_buffer buf = {0};
    unsigned long index = 0;
    if (!PyArg_ParseTuple(args, "iy*k", &op, &buf, &index))
        return NULL;
    int64_t rc;
    if (op == 0 || op == 1) {
        char path[96];
        memset(path, 0, sizeof(path));
        if (op == 0) {
            rc = infinity_system_call(0x12e0, index, (uintptr_t)path, 96, 0, 0, 0);
            PyBuffer_Release(&buf);
            if (rc < 0 || rc > 95)
                Py_RETURN_NONE;
            return PyUnicode_DecodeUTF8(path, rc, "replace");
        }
        if (buf.len > 95) {
            PyBuffer_Release(&buf);
            PyErr_SetString(PyExc_ValueError, "Invalid path");
            return NULL;
        }
        rc = infinity_system_call(0x12e1, (uintptr_t)buf.buf, buf.len, 0, 0, 0, 0);
        if (rc > 0 && rc <= MEDIA_LIMIT) {
            PyObject *result = PyBytes_FromStringAndSize(NULL, rc);
            if (!result) {
                PyBuffer_Release(&buf);
                return NULL;
            }
            int64_t n = infinity_system_call(0x12e1, (uintptr_t)buf.buf, buf.len,
                                             (uintptr_t)PyBytes_AS_STRING(result), rc, 0, 0);
            if (n == rc) {
                PyBuffer_Release(&buf);
                return result;
            }
            Py_DECREF(result);
        }
        PyBuffer_Release(&buf);
        PyErr_SetString(PyExc_OSError,
                        "Media file unavailable or exceeds the object-store media limit");
        return NULL;
    }
    if (op == 6) {
        static const char header[] = "P6\n96 34\n255\n";
        PyObject *out = PyBytes_FromStringAndSize(NULL, sizeof(header) - 1 + 96 * 34 * 3);
        PyBuffer_Release(&buf);
        if (!out)
            return NULL;
        memcpy(PyBytes_AS_STRING(out), header, sizeof(header) - 1);
        rc = infinity_system_call(0x12e6, (uintptr_t)PyBytes_AS_STRING(out) + sizeof(header) - 1,
                                  index, 0, 0, 0, 0);
        if (rc == 0)
            return out;
        Py_DECREF(out);
        PyErr_SetString(PyExc_OSError, "UIKit surface unavailable");
        return NULL;
    }
    rc = (op >= 2 && op <= 5)
             ? infinity_system_call(0x12e0 + op, (uintptr_t)buf.buf, buf.len, index, 0, 0, 0)
             : -22;
    PyBuffer_Release(&buf);
    return PyLong_FromLongLong(rc);
}
// ------------------------=
// FUNC: seek_stream
// DESC: Seeks the selected stream to an earlier keyframe and resets decoder delay.
// ------------------=
static PyObject *seek_stream(PyObject *self, PyObject *args) {
    PyObject *cap;
    double seconds;
    if (!PyArg_ParseTuple(args, "Od", &cap, &seconds))
        return NULL;
    Stream *s = PyCapsule_GetPointer(cap, CAP);
    if (!s)
        return NULL;
    if (!(seconds >= 0 && seconds < 86400 * 30)) {
        PyErr_SetString(PyExc_ValueError, "Invalid seek time");
        return NULL;
    }
    AVStream *st = s->format->streams[s->stream];
    int64_t ts = av_rescale_q((int64_t)(seconds * AV_TIME_BASE), AV_TIME_BASE_Q, st->time_base);
    if (st->start_time != AV_NOPTS_VALUE)
        ts += st->start_time;
    if (av_seek_frame(s->format, s->stream, ts, AVSEEK_FLAG_BACKWARD) < 0) {
        PyErr_SetString(PyExc_ValueError, "Cannot seek this track");
        return NULL;
    }
    avcodec_flush_buffers(s->codec);
    swr_free(&s->resample);
    av_packet_unref(s->packet);
    s->eof = 0;
    Py_RETURN_NONE;
}
static PyMethodDef methods[] = {{"seek", seek_stream, METH_VARARGS, NULL},
                                {"open", open_stream, METH_VARARGS, NULL},
                                {"next", next_frame, METH_O, NULL},
                                {"io", media_io, METH_VARARGS, NULL},
                                {NULL}};
static struct PyModuleDef module = {PyModuleDef_HEAD_INIT, "_infinity_media", NULL, -1, methods};
// ------------------------=
// FUNC: PyInit__infinity_media
// DESC: Registers the isolated media decoder and native output transport.
// ------------------=
PyMODINIT_FUNC PyInit__infinity_media(void) { return PyModule_Create(&module); }
