#include <boost/polygon/polygon.hpp>
#include <boost/version.hpp>

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <limits>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <vector>

namespace bp = boost::polygon;
using namespace boost::polygon::operators;

enum class Operation : std::uint8_t { Xor = 0, Union = 1, Intersect = 2, Difference = 3 };
enum class OutputKind { Shapes, Contours };

struct Config {
    std::string input;
    OutputKind output_kind = OutputKind::Shapes;
    std::chrono::milliseconds budget{250};
    std::size_t samples = 7;
};

template <typename Coordinate>
using RawContour = std::vector<bp::point_data<Coordinate>>;

template <typename Coordinate>
struct Case {
    Operation operation;
    std::vector<RawContour<Coordinate>> subject;
    std::vector<RawContour<Coordinate>> clip;
};

struct Result {
    std::size_t iterations_per_sample = 0;
    std::vector<std::uint64_t> samples_ns;
    std::size_t shapes = 0;
    std::size_t contours = 0;
    std::size_t points = 0;
    std::int64_t area = 0;
};

static volatile std::size_t output_sink = 0;

template <typename T>
T read_value(std::istream& input) {
    T value{};
    input.read(reinterpret_cast<char*>(&value), sizeof(T));
    if (!input) {
        throw std::runtime_error("unexpected end of case file");
    }
#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_BIG_ENDIAN__
    std::reverse(reinterpret_cast<char*>(&value), reinterpret_cast<char*>(&value) + sizeof(T));
#endif
    return value;
}

template <typename Coordinate>
std::vector<RawContour<Coordinate>> read_contours(std::istream& input, std::uint64_t count) {
    std::vector<RawContour<Coordinate>> contours;
    contours.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t contour_index = 0; contour_index < count; ++contour_index) {
        const auto point_count = read_value<std::uint64_t>(input);
        RawContour<Coordinate> contour;
        contour.reserve(static_cast<std::size_t>(point_count));
        for (std::uint64_t point_index = 0; point_index < point_count; ++point_index) {
            const auto x = read_value<Coordinate>(input);
            const auto y = read_value<Coordinate>(input);
            contour.emplace_back(x, y);
        }
        contours.push_back(std::move(contour));
    }
    return contours;
}

template <typename Coordinate>
Case<Coordinate> read_case(std::istream& input, Operation operation) {
    const auto subject_count = read_value<std::uint64_t>(input);
    const auto clip_count = read_value<std::uint64_t>(input);
    return {
        .operation = operation,
        .subject = read_contours<Coordinate>(input, subject_count),
        .clip = read_contours<Coordinate>(input, clip_count),
    };
}

template <typename Coordinate>
bp::polygon_90_set_data<Coordinate> make_set(const std::vector<RawContour<Coordinate>>& contours) {
    bp::polygon_90_set_data<Coordinate> result{bp::HORIZONTAL};
    for (const auto& raw : contours) {
        bp::polygon_90_data<Coordinate> polygon;
        bp::set_points(polygon, raw.begin(), raw.end());
        result.insert(polygon);
    }
    return result;
}

template <typename Coordinate>
bp::polygon_90_set_data<Coordinate> apply_operation(
    const bp::polygon_90_set_data<Coordinate>& subject,
    const bp::polygon_90_set_data<Coordinate>& clip,
    Operation operation
) {
    switch (operation) {
        case Operation::Xor: return subject ^ clip;
        case Operation::Union: return subject | clip;
        case Operation::Intersect: return subject & clip;
        case Operation::Difference: return subject - clip;
    }
    std::abort();
}

template <typename Polygon>
std::size_t point_count(const Polygon& polygon) {
    return static_cast<std::size_t>(std::distance(bp::begin_points(polygon), bp::end_points(polygon)));
}

template <typename Coordinate>
Result run(const Case<Coordinate>& workload, OutputKind output_kind, const Config& config) {
    using Polygon = bp::polygon_90_with_holes_data<Coordinate>;
    using Contour = bp::polygon_90_data<Coordinate>;
    using PolygonSet = bp::polygon_90_set_data<Coordinate>;

    auto solve_shapes = [&]() {
        const auto subject = make_set(workload.subject);
        const auto clip = make_set(workload.clip);
        auto result = apply_operation(subject, clip, workload.operation);
        std::vector<Polygon> output;
        result.get(output);
        return output;
    };
    auto solve_contours = [&]() {
        const auto subject = make_set(workload.subject);
        const auto clip = make_set(workload.clip);
        auto result = apply_operation(subject, clip, workload.operation);
        std::vector<Contour> output;
        result.get(output);
        return output;
    };

    Result result;
    {
        const auto subject = make_set(workload.subject);
        const auto clip = make_set(workload.clip);
        PolygonSet validation = apply_operation(subject, clip, workload.operation);
        result.area = static_cast<std::int64_t>(bp::area(validation));
        if (output_kind == OutputKind::Shapes) {
            std::vector<Polygon> output;
            validation.get(output);
            result.shapes = output.size();
            for (const auto& polygon : output) {
                ++result.contours;
                result.points += point_count(polygon);
                for (auto hole = bp::begin_holes(polygon); hole != bp::end_holes(polygon); ++hole) {
                    ++result.contours;
                    result.points += point_count(*hole);
                }
            }
        } else {
            std::vector<Contour> output;
            validation.get(output);
            result.contours = output.size();
            for (const auto& contour : output) {
                result.points += point_count(contour);
            }
        }
    }

    const auto target = config.budget / static_cast<std::int64_t>(config.samples);
    std::chrono::nanoseconds calibration{1};
    if (output_kind == OutputKind::Shapes) {
        const auto start = std::chrono::steady_clock::now();
        const auto output = solve_shapes();
        output_sink ^= output.size();
        calibration = std::max(
            std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now() - start),
            std::chrono::nanoseconds{1}
        );
    } else {
        const auto start = std::chrono::steady_clock::now();
        const auto output = solve_contours();
        output_sink ^= output.size();
        calibration = std::max(
            std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now() - start),
            std::chrono::nanoseconds{1}
        );
    }
    const auto raw_iterations = std::max<std::int64_t>(1, target.count() * 1'000'000 / calibration.count());
    result.iterations_per_sample = static_cast<std::size_t>(std::min<std::int64_t>(1'000'000, raw_iterations));
    result.samples_ns.reserve(config.samples);
    for (std::size_t sample = 0; sample < config.samples; ++sample) {
        const auto start = std::chrono::steady_clock::now();
        for (std::size_t iteration = 0; iteration < result.iterations_per_sample; ++iteration) {
            if (output_kind == OutputKind::Shapes) {
                const auto output = solve_shapes();
                output_sink ^= output.size();
            } else {
                const auto output = solve_contours();
                output_sink ^= output.size();
            }
        }
        const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
            std::chrono::steady_clock::now() - start
        );
        result.samples_ns.push_back(
            static_cast<std::uint64_t>(elapsed.count() / static_cast<std::int64_t>(result.iterations_per_sample))
        );
    }
    return result;
}

Config parse_config(int argc, char** argv) {
    Config config;
    for (int index = 1; index < argc; ++index) {
        const std::string_view option{argv[index]};
        if (option == "--input" && index + 1 < argc) {
            config.input = argv[++index];
        } else if (option == "--output-kind" && index + 1 < argc) {
            const std::string_view value{argv[++index]};
            config.output_kind = value == "contours" ? OutputKind::Contours : OutputKind::Shapes;
        } else if (option == "--budget-ms" && index + 1 < argc) {
            config.budget = std::chrono::milliseconds{std::stoll(argv[++index])};
        } else if (option == "--samples" && index + 1 < argc) {
            config.samples = static_cast<std::size_t>(std::stoull(argv[++index]));
        } else {
            throw std::invalid_argument("unknown or incomplete option: " + std::string{option});
        }
    }
    if (config.input.empty() || config.samples == 0 || config.budget.count() <= 0) {
        throw std::invalid_argument("input, positive samples, and positive budget are required");
    }
    return config;
}

void print_json(const Result& result, OutputKind output_kind) {
    const auto major = BOOST_VERSION / 100000;
    const auto minor = BOOST_VERSION / 100 % 1000;
    const auto patch = BOOST_VERSION % 100;
    std::cout << "{\n"
              << "  \"boost_version\": \"" << major << '.' << minor << '.' << patch << "\",\n"
              << "  \"iterations_per_sample\": " << result.iterations_per_sample << ",\n"
              << "  \"samples_ns\": [";
    for (std::size_t index = 0; index < result.samples_ns.size(); ++index) {
        if (index != 0) std::cout << ", ";
        std::cout << result.samples_ns[index];
    }
    std::cout << "],\n  \"shapes\": ";
    if (output_kind == OutputKind::Shapes) std::cout << result.shapes;
    else std::cout << "null";
    std::cout << ",\n  \"contours\": " << result.contours
              << ",\n  \"points\": " << result.points
              << ",\n  \"area_two\": " << result.area * 2
              << "\n}\n";
}

int main(int argc, char** argv) try {
    const auto config = parse_config(argc, argv);
    std::ifstream input(config.input, std::ios::binary);
    if (!input) throw std::runtime_error("unable to open case file");
    char magic[8]{};
    input.read(magic, 8);
    if (std::string_view{magic, 8} != "XOVCASE1") throw std::runtime_error("invalid case file");
    const auto width = read_value<std::uint8_t>(input);
    const auto operation = static_cast<Operation>(read_value<std::uint8_t>(input));
    read_value<std::uint8_t>(input);
    read_value<std::uint8_t>(input);
    Result result;
    if (width == 4) {
        result = run(read_case<std::int32_t>(input, operation), config.output_kind, config);
    } else if (width == 8) {
        result = run(read_case<std::int64_t>(input, operation), config.output_kind, config);
    } else {
        throw std::runtime_error("unsupported coordinate width");
    }
    print_json(result, config.output_kind);
    return EXIT_SUCCESS;
} catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return EXIT_FAILURE;
}
