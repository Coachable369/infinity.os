// Independent host diagnostic using the already-vendored upstream llama implementation.
// It is not shipped or used as an InfinityOS execution backend.
#include "llama.h"
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

// ------------------------=
// FUNC: main
// DESC: Compares exact pinned Hermes ChatML, token IDs, greedy logits, and generated output with the native implementation.
// ------------------=
int main(int argc, char **argv) {
    if (argc != 5 && argc != 6) { std::fprintf(stderr,"usage: hermes-reference MODEL PROMPT OUTPUT_PREFIX MAX_TOKENS [native|system]\n"); return 2; }
    std::ifstream input(argv[2],std::ios::binary);
    std::string text((std::istreambuf_iterator<char>(input)),std::istreambuf_iterator<char>());
    if (!input || text.empty()) return 3;
    const std::string prefix=argv[3];
    const int maximum=std::stoi(argv[4]);
    if (maximum<0 || maximum>4096 || prefix.rfind("build/",0)!=0 || prefix.find("..")!=std::string::npos) return 4;
    llama_backend_init();
    auto mp=llama_model_default_params();mp.n_gpu_layers=0;
    llama_model *model=llama_model_load_from_file(argv[1],mp);
    if (!model) return 5;
    const llama_vocab *vocab=llama_model_get_vocab(model);
    const std::string framing=argc==6?argv[5]:"native";
    if(framing!="native" && framing!="system") return 4;
    const char *chat_template=llama_model_chat_template(model,nullptr);
    std::ofstream metadata(prefix+".chat-template.txt");
    if(chat_template) metadata<<chat_template;
    std::string prompt="<|im_start|>user\n"+text+"<|im_end|>\n<|im_start|>assistant\n";
    if(framing=="system") {
        const std::string marker="\nUSER REQUEST:\n";
        const auto split=text.rfind(marker);
        if(split==std::string::npos) return 4;
        prompt="<|im_start|>system\n"+text.substr(0,split)+"<|im_end|>\n<|im_start|>user\n"+text.substr(split+marker.size())+"<|im_end|>\n<|im_start|>assistant\n";
    }
    std::ofstream framed(prefix+".framed-prompt.txt",std::ios::binary);framed<<prompt;framed.close();
    std::vector<llama_token> tokens(4096);
    int count=llama_tokenize(vocab,prompt.data(),int(prompt.size()),tokens.data(),int(tokens.size()),false,true);
    if(count<1 || count>=4096) return 6;
    std::ofstream prompt_ids(prefix+".prompt-ids.txt");
    for(int i=0;i<count;i++) prompt_ids<<tokens[i]<<'\n';
    prompt_ids.close();
    std::fprintf(stderr,"reference prompt_tokens=%d\n",count);
    if(maximum==0) {llama_model_free(model);llama_backend_free();return 0;}
    auto cp=llama_context_default_params();
    cp.n_ctx=4096;cp.n_batch=512;cp.n_ubatch=512;cp.n_threads=4;cp.n_threads_batch=4;
    cp.type_k=GGML_TYPE_F32;cp.type_v=GGML_TYPE_F32;
    cp.flash_attn_type=LLAMA_FLASH_ATTN_TYPE_DISABLED;
    llama_context *context=llama_init_from_model(model,cp);
    if(!context) return 7;
    const auto started=std::chrono::steady_clock::now();
    for(int offset=0;offset<count;offset+=512) {
        auto batch=llama_batch_get_one(tokens.data()+offset,std::min(512,count-offset));
        if(llama_decode(context,batch)!=0) return 8;
    }
    std::ofstream output(prefix+".output.txt",std::ios::binary);
    std::ofstream ids(prefix+".generated-ids.txt");
    const int vocabulary=llama_vocab_n_tokens(vocab);
    for(int generated=0;generated<maximum && count+generated<4096;generated++) {
        float *logits=llama_get_logits_ith(context,-1);
        if(!logits) return 9;
        if(generated==0) {
            std::ofstream binary(prefix+".first-logits.f32",std::ios::binary);
            binary.write(reinterpret_cast<const char *>(logits),vocabulary*sizeof(float));
        }
        llama_token best=llama_token(std::max_element(logits,logits+vocabulary)-logits);
        ids<<best<<'\n';ids.flush();
        if(llama_vocab_is_eog(vocab,best)) break;
        char piece[2048];int length=llama_token_to_piece(vocab,best,piece,sizeof(piece),0,true);
        if(length<0) return 10;
        output.write(piece,length);output.flush();
        std::fwrite(piece,1,length,stdout);std::fflush(stdout);
        auto batch=llama_batch_get_one(&best,1);
        if(llama_decode(context,batch)!=0) return 11;
    }
    std::fprintf(stderr,"\nreference complete elapsed_ms=%lld\n",(long long)std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now()-started).count());
    llama_free(context);llama_model_free(model);llama_backend_free();return 0;
}
