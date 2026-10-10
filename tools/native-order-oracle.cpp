#define NOMINMAX
#include <windows.h>
#include "ot2/native_order_sort.hpp"
#include "ot2/runtime.hpp"
#include "native_order_sort_fixture.hpp"
#include <algorithm>
#include <cstring>
#include <iostream>
#include <random>

namespace {
struct Row{std::int32_t key,id;std::array<unsigned char,56> payload;};
static_assert(sizeof(Row)==64);
using Pairs=std::vector<std::pair<std::int32_t,std::int32_t>>;
struct Trace{Pairs calls;bool reverse=false;};
bool compare(void* owner,const Row* a,const Row* b){
    auto& trace=*static_cast<Trace*>(owner);trace.calls.emplace_back(a->id,b->id);
    return trace.reverse?a->key>b->key:a->key<b->key;
}
float native_log(float value){return std::log(value);}
void* native_memset(void* dest,int value,std::size_t count){return std::memset(dest,value,count);}
struct ShuffleTrace{
    ot2::GameplayRng rng;
    std::vector<std::int32_t> visits;
    std::vector<std::uint32_t> weights;
    // Interleave visits and RNG calls to detect timing differences too.
    std::vector<std::uint64_t> events;
};
thread_local ShuffleTrace* activeShuffle=nullptr;
bool eligible(void* owner,Row* row){
    auto& trace=*static_cast<ShuffleTrace*>(owner);
    trace.visits.push_back(row->id);trace.events.push_back(static_cast<std::uint32_t>(row->id));
    const bool result=row->key!=0;row->id=-1234; // Native passes a copied record.
    return result;
}
bool weighted(float value){
    std::uint32_t bits;std::memcpy(&bits,&value,4);
    activeShuffle->weights.push_back(bits);activeShuffle->events.push_back(0x100000000ull+bits);
    return activeShuffle->rng.weighted_bool(value);
}
void native_swap(void* a,void* b,std::size_t count){
    if(count!=64)std::terminate();
    unsigned char temp[64];std::memcpy(temp,a,64);std::memcpy(a,b,64);std::memcpy(b,temp,64);
}
class Reference{
    unsigned char* code_=nullptr;
public:
    Reference(){
        code_=static_cast<unsigned char*>(VirtualAlloc(nullptr,sizeof(fixture::nativeSortCode),MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE));
        if(!code_)throw std::runtime_error("sort oracle allocation failed");
        std::memcpy(code_,fixture::nativeSortCode,sizeof(fixture::nativeSortCode));
        auto patch=[&](unsigned offset,auto fn){const auto address=reinterpret_cast<std::uintptr_t>(fn);std::memcpy(code_+offset,&address,sizeof(address));};
        patch(fixture::nativeSortCompareWaitOffset,&compare);patch(fixture::nativeSortCompareNextOffset,&compare);
        patch(fixture::nativeSortLogOffset,&native_log);patch(fixture::nativeSortMemsetOffset,&native_memset);
        patch(fixture::nativeSortEligibleOffset,&eligible);patch(fixture::nativeSortWeightedOffset,&weighted);patch(fixture::nativeSortSwapOffset,&native_swap);
        DWORD old=0;
        if(!VirtualProtect(code_,sizeof(fixture::nativeSortCode),PAGE_EXECUTE_READ,&old)||
           !FlushInstructionCache(GetCurrentProcess(),code_,sizeof(fixture::nativeSortCode))){
            VirtualFree(code_,0,MEM_RELEASE);throw std::runtime_error("sort oracle protection failed");
        }
    }
    Reference(const Reference&)=delete;Reference& operator=(const Reference&)=delete;
    ~Reference(){VirtualFree(code_,0,MEM_RELEASE);}
    void run(unsigned offset,std::vector<Row>& rows,Trace& trace)const{
        void* owner=&trace;
        reinterpret_cast<void(*)(Row*,std::int32_t,std::uint8_t,void**)>(code_+offset)(rows.data(),static_cast<std::int32_t>(rows.size()),0,&owner);
    }
    bool shuffle(std::vector<Row>& rows,ShuffleTrace& trace)const{
        struct NativeArray{Row* data;std::int32_t size,capacity;};
        NativeArray array{rows.data(),static_cast<std::int32_t>(rows.size()),static_cast<std::int32_t>(rows.capacity())};
        activeShuffle=&trace;
        return reinterpret_cast<bool(*)(void*,NativeArray*)>(code_+fixture::nativeSortShuffleOffset)(&trace,&array);
    }
};
void check(bool ok,const char* message){if(!ok)throw std::runtime_error(message);}
}
int main(){Reference reference;std::cout<<"[";bool first=true;
for(int n=0;n<=64;++n)for(int pattern=0;pattern<5;++pattern){
std::vector<Row> rows;for(int i=0;i<n;++i)rows.push_back(Row{pattern==0?0:pattern==1?i:pattern==2?n-i:pattern==3?i%3:(i*37+11)%17,i,{}});
if(!first)std::cout<<",";first=false;std::cout<<"{\"speeds\":[";
for(int i=0;i<n;++i){if(i)std::cout<<",";std::cout<<rows[i].key;}std::cout<<"],\"order\":[";
Trace trace;trace.reverse=true;reference.run(fixture::nativeSortSortWaitOffset,rows,trace);
for(int i=0;i<n;++i){if(i)std::cout<<",";std::cout<<rows[i].id;}std::cout<<"]}";
}std::cout<<"]";}
