#pragma once
#include <memory>
#include <cstdint>

#include "rust/cxx.h"
#include <libraw/libraw.h>

class RawProcessor {
private:
    LibRaw processor;
    libraw_processed_image_t* image;

public:
    RawProcessor();
    ~RawProcessor();

    void open_and_process(rust::String path);
    uint16_t get_width() const;
    uint16_t get_height() const;
    uint16_t get_bits() const;
    uint32_t get_data_size() const;
    void copy_data_to_buffer_u8(rust::Slice<uint8_t> buffer) const;
    void copy_data_to_buffer_u16(rust::Slice<uint16_t> buffer) const;
};

std::unique_ptr<RawProcessor> new_raw_processor();
