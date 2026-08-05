CXX ?= clang++
BOOST_PREFIX ?= $(shell brew --prefix boost 2>/dev/null)

TARGET := build/boost_polygon_bench
SOURCE := src/main.cpp

CPPFLAGS += -I$(BOOST_PREFIX)/include
CXXFLAGS ?= -O3 -DNDEBUG -std=c++20 -march=native

.PHONY: all run compare large clean

all: $(TARGET)

$(TARGET): $(SOURCE)
	@mkdir -p build
	$(CXX) $(CPPFLAGS) $(CXXFLAGS) $(SOURCE) -o $(TARGET)

run: $(TARGET)
	./$(TARGET) --scenario all --budget-ms 1000

compare: $(TARGET)
	cargo run --manifest-path rust/Cargo.toml --release -- --scenario all --budget-ms 600
	./$(TARGET) --scenario all --budget-ms 600

large: $(TARGET)
	cargo run --manifest-path rust/Cargo.toml --release -- --large-contours
	./$(TARGET) --large-contours

clean:
	rm -rf build
