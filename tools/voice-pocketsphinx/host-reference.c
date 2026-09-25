/* Host-only reference for comparing the identical upstream decoder configuration.
 * Never linked into InfinityOS or used to deliver guest transcripts. */
#include <pocketsphinx.h>
#include <stdio.h>
#include <assert.h>
// ------------------------=
// FUNC: main
// DESC: Runs the same acoustic/language model configuration as the native probe for differential diagnosis.
// ------------------=
int main(int argc, char **argv){
    ps_config_t*c=ps_config_init(NULL);
    ps_config_set_str(c,"hmm","build/voice-pocketsphinx-src/model/en-us/en-us");
    ps_config_set_str(c,"lm","build/voice-pocketsphinx-src/model/en-us/en-us.lm.bin");
    ps_config_set_str(c,"dict","build/voice-pocketsphinx-src/model/en-us/cmudict-en-us.dict");
    ps_config_set_float(c,"samprate",16000);ps_config_set_bool(c,"mmap",1);
    ps_decoder_t*d=ps_init(c);assert(d);ps_config_free(c);
    FILE*f=fopen(argc>1?argv[1]:"build/voice-pocketsphinx-src/test/data/goforward.raw","rb");assert(f);
    short pcm[160000];size_t n;assert(ps_start_utt(d)==0);
    if(argc>2){
        n=fread(pcm,2,160000,f);assert(n>0&&fgetc(f)==EOF);
        assert(ps_process_raw(d,pcm,n,0,1)>=0);
    }else{
        while((n=fread(pcm,2,1600,f)))assert(ps_process_raw(d,pcm,n,0,0)>=0);
    }
    assert(ps_end_utt(d)==0);puts(ps_get_hyp(d,NULL));ps_free(d);fclose(f);
}
