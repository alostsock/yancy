#include <memory>
#include <stdexcept>

#include "yancy/src/raw_processor.h"

RawProcessor::RawProcessor() : image(nullptr) {}

RawProcessor::~RawProcessor() {
    if (image) {
        LibRaw::dcraw_clear_mem(image);
    }
}

void RawProcessor::open_and_process(rust::String path) {
    if (image) {
        LibRaw::dcraw_clear_mem(image);
        image = nullptr;
    }

    #define OUT processor.imgdata.params

    OUT.output_bps = 16;
    OUT.gamm[0] = 1.0;
    OUT.gamm[1] = 1.0;
    OUT.use_camera_wb = 1;
    OUT.use_camera_matrix = 1;
    OUT.no_auto_bright = 1;
    OUT.adjust_maximum_thr = 0.0;

    int ret = processor.open_file(path.c_str());
    if (ret != LIBRAW_SUCCESS) {
        throw std::runtime_error(std::string("Failed to open file: ") +
                               libraw_strerror(ret));
    }

    ret = processor.unpack();
    if (ret != LIBRAW_SUCCESS) {
        throw std::runtime_error(std::string("Failed to unpack RAW data: ") +
                               libraw_strerror(ret));
    }

    ret = processor.dcraw_process();
    if (ret != LIBRAW_SUCCESS) {
        throw std::runtime_error(std::string("Failed to process image: ") +
                               libraw_strerror(ret));
    }

    image = processor.dcraw_make_mem_image(&ret);
    if (!image) {
        throw std::runtime_error(std::string("Failed to create memory image: ") +
                               libraw_strerror(ret));
    }

    if (image->type != LIBRAW_IMAGE_BITMAP) {
        throw std::runtime_error("Image is not a bitmap");
    }

    if (image->colors != 3) {
        throw std::runtime_error("Image is not RGB (expected 3 colors)");
    }
}

uint16_t RawProcessor::get_width() const {
    if (!image) {
        throw std::runtime_error("No image loaded");
    }
    return image->width;
}

uint16_t RawProcessor::get_height() const {
    if (!image) {
        throw std::runtime_error("No image loaded");
    }
    return image->height;
}

uint16_t RawProcessor::get_bits() const {
    if (!image) {
        throw std::runtime_error("No image loaded");
    }
    return image->bits;
}

uint32_t RawProcessor::get_data_size() const {
    if (!image) {
        throw std::runtime_error("No image loaded");
    }
    return image->data_size;
}

void RawProcessor::copy_data_to_buffer_u8(rust::Slice<uint8_t> buffer) const {
    if (!image) {
        throw std::runtime_error("No image loaded");
    }

    if (image->bits != 8) {
        throw std::runtime_error("Expected bit depth of 8");
    }

    if (buffer.size() != image->data_size) {
        throw std::runtime_error("Buffer size mismatch. Expected " +
                               std::to_string(image->data_size) +
                               " bytes, got " +
                               std::to_string(buffer.size()) + " bytes");
    }

    ::memcpy(buffer.data(), image->data, image->data_size);
}

void RawProcessor::copy_data_to_buffer_u16(rust::Slice<uint16_t> buffer) const {
    if (!image) {
        throw std::runtime_error("No image loaded");
    }

    if (image->bits != 16) {
        throw std::runtime_error("Expected bit depth of 16");
    }

    if (buffer.size() != image->data_size) {
        throw std::runtime_error("Buffer size mismatch. Expected " +
                               std::to_string(image->data_size) +
                               " bytes, got " +
                               std::to_string(buffer.size()) + " bytes");
    }

    ::memcpy(buffer.data(), image->data, image->data_size);
}

std::unique_ptr<RawProcessor> new_raw_processor() {
  return std::make_unique<RawProcessor>();
}
