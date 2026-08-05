#include <boost/polygon/polygon.hpp>
#include <boost/version.hpp>

#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <vector>

namespace bp = boost::polygon;
using namespace boost::polygon::operators;

using Coordinate = std::int32_t;
using Rectangle = bp::rectangle_data<Coordinate>;
using Polygon = bp::polygon_90_with_holes_data<Coordinate>;
using PolygonSet = bp::polygon_90_set_data<Coordinate>;
using Polygons = std::vector<Polygon>;

static_assert(std::is_same_v<
              typename bp::geometry_concept<PolygonSet>::type,
              bp::polygon_90_set_concept>);

enum class Operation {
    xor_,
    union_,
    intersection,
    difference,
};

struct Config {
    std::string scenario = "all";
    std::size_t n_override = 0;
    std::chrono::milliseconds budget{600};
};

struct Workload {
    std::string name;
    std::size_t n;
    Operation operation;
    std::vector<Rectangle> subject;
    std::vector<Rectangle> clip;
    std::int64_t expected_area;
};

struct Measurement {
    std::size_t iterations;
    std::chrono::nanoseconds elapsed;

    [[nodiscard]] double milliseconds_per_iteration() const {
        return static_cast<double>(elapsed.count()) / 1'000'000.0 /
               static_cast<double>(iterations);
    }
};

static volatile std::size_t output_sink = 0;

[[nodiscard]] std::size_t parse_size(std::string_view text, std::string_view option) {
    std::size_t parsed = 0;
    const auto value = std::stoull(std::string{text}, &parsed);
    if (parsed != text.size() || value == 0) {
        throw std::invalid_argument(std::string{option} + " must be a positive integer");
    }
    return value;
}

[[nodiscard]] Config parse_config(int argc, char** argv) {
    Config config;
    for (int index = 1; index < argc; ++index) {
        const std::string_view option{argv[index]};
        if (option == "--scenario" && index + 1 < argc) {
            config.scenario = argv[++index];
        } else if (option == "--n" && index + 1 < argc) {
            config.n_override = parse_size(argv[++index], option);
        } else if (option == "--budget-ms" && index + 1 < argc) {
            config.budget = std::chrono::milliseconds{parse_size(argv[++index], option)};
        } else if (option == "--help") {
            std::cout
                << "Usage: boost_polygon_bench [--scenario NAME] [--n N] [--budget-ms MS]\n"
                << "Scenarios: all, checkerboard, not-overlap, lines-net, windows, nested\n";
            std::exit(EXIT_SUCCESS);
        } else {
            throw std::invalid_argument("unknown or incomplete option: " + std::string{option});
        }
    }
    if (config.scenario == "all" && config.n_override != 0) {
        throw std::invalid_argument("--n can only be used with one selected scenario");
    }
    return config;
}

[[nodiscard]] std::vector<Rectangle> many_squares(
    Coordinate origin_x,
    Coordinate origin_y,
    Coordinate size,
    Coordinate step,
    std::size_t n
) {
    std::vector<Rectangle> result;
    result.reserve(n * n);
    for (std::size_t row = 0; row < n; ++row) {
        const auto y = origin_y + static_cast<Coordinate>(row) * step;
        for (std::size_t column = 0; column < n; ++column) {
            const auto x = origin_x + static_cast<Coordinate>(column) * step;
            result.emplace_back(x, y, x + size, y + size);
        }
    }
    return result;
}

[[nodiscard]] Workload checkerboard(std::size_t n) {
    const auto count = static_cast<std::int64_t>(n);
    const auto clip_count = static_cast<std::int64_t>(n - 1);
    return {
        .name = "Checkerboard / XOR",
        .n = n,
        .operation = Operation::xor_,
        .subject = many_squares(0, 0, 20, 30, n),
        .clip = many_squares(15, 15, 20, 30, n - 1),
        // Every clip square overlaps four subject squares by 5x5.
        .expected_area = 400 * count * count + 200 * clip_count * clip_count,
    };
}

[[nodiscard]] Workload not_overlap(std::size_t n) {
    const auto count = static_cast<std::int64_t>(n);
    const auto clip_count = static_cast<std::int64_t>(n - 1);
    return {
        .name = "Not Overlap / Union",
        .n = n,
        .operation = Operation::union_,
        .subject = many_squares(0, 0, 10, 30, n),
        .clip = many_squares(15, 15, 10, 30, n - 1),
        .expected_area = 100 * (count * count + clip_count * clip_count),
    };
}

[[nodiscard]] std::vector<Rectangle> many_lines_x(Coordinate spacing, std::size_t n) {
    std::vector<Rectangle> result;
    result.reserve(n);
    const auto width = spacing / 2;
    const auto span = spacing * static_cast<Coordinate>(n) / 2;
    auto x = -span + width / 2;
    for (std::size_t index = 0; index < n; ++index) {
        result.emplace_back(x, -span, x + width, span);
        x += spacing;
    }
    return result;
}

[[nodiscard]] std::vector<Rectangle> many_lines_y(Coordinate spacing, std::size_t n) {
    std::vector<Rectangle> result;
    result.reserve(n);
    const auto height = spacing / 2;
    const auto span = spacing * static_cast<Coordinate>(n) / 2;
    auto y = -span + height / 2;
    for (std::size_t index = 0; index < n; ++index) {
        result.emplace_back(-span, y - height, span, y);
        y += spacing;
    }
    return result;
}

[[nodiscard]] Workload lines_net(std::size_t n) {
    const auto count = static_cast<std::int64_t>(n);
    return {
        .name = "Lines Net / Intersection",
        .n = n,
        .operation = Operation::intersection,
        .subject = many_lines_x(20, n),
        .clip = many_lines_y(20, n),
        // The first horizontal line extends half its height below the vertical span.
        .expected_area = 100 * count * count - 50 * count,
    };
}

[[nodiscard]] Workload windows(std::size_t n) {
    constexpr Coordinate outer = 20;
    constexpr Coordinate inner = 10;
    constexpr Coordinate step = 30;
    const auto origin = -static_cast<Coordinate>(n) * step / 2;
    const auto inset = (outer - inner) / 2;

    std::vector<Rectangle> subject;
    std::vector<Rectangle> clip;
    subject.reserve(n * n);
    clip.reserve(n * n);
    for (std::size_t row = 0; row < n; ++row) {
        const auto y = origin + static_cast<Coordinate>(row) * step;
        for (std::size_t column = 0; column < n; ++column) {
            const auto x = origin + static_cast<Coordinate>(column) * step;
            subject.emplace_back(x, y, x + outer, y + outer);
            clip.emplace_back(
                x + inset,
                y + inset,
                x + inset + inner,
                y + inset + inner
            );
        }
    }

    const auto count = static_cast<std::int64_t>(n);
    return {
        .name = "Windows / Difference",
        .n = n,
        .operation = Operation::difference,
        .subject = std::move(subject),
        .clip = std::move(clip),
        .expected_area = 300 * count * count,
    };
}

[[nodiscard]] Workload nested_squares(std::size_t n) {
    constexpr Coordinate thickness = 4;
    constexpr Coordinate step = 8;
    std::vector<Rectangle> vertical;
    std::vector<Rectangle> horizontal;
    vertical.reserve(2 * n);
    horizontal.reserve(2 * n);

    Coordinate radius = step;
    for (std::size_t index = 0; index < n; ++index) {
        horizontal.emplace_back(-radius, radius - thickness, radius, radius);
        horizontal.emplace_back(-radius, -radius, radius, -radius + thickness);
        vertical.emplace_back(-radius, -radius, -radius + thickness, radius);
        vertical.emplace_back(radius - thickness, -radius, radius, radius);
        radius += step;
    }

    const auto count = static_cast<std::int64_t>(n);
    return {
        .name = "Nested Squares / XOR",
        .n = n,
        .operation = Operation::xor_,
        .subject = std::move(vertical),
        .clip = std::move(horizontal),
        .expected_area = 128 * count * count,
    };
}

[[nodiscard]] Workload make_workload(std::string_view scenario, std::size_t n) {
    if (scenario == "checkerboard") {
        return checkerboard(n);
    }
    if (scenario == "not-overlap") {
        return not_overlap(n);
    }
    if (scenario == "lines-net") {
        return lines_net(n);
    }
    if (scenario == "windows") {
        return windows(n);
    }
    if (scenario == "nested") {
        return nested_squares(n);
    }
    throw std::invalid_argument("unknown scenario: " + std::string{scenario});
}

[[nodiscard]] std::vector<Workload> make_workloads(const Config& config) {
    if (config.scenario != "all") {
        const auto default_n = config.scenario == "nested" ? 4096u : 128u;
        return {make_workload(
            config.scenario,
            config.n_override == 0 ? default_n : config.n_override
        )};
    }

    std::vector<Workload> result;
    result.push_back(checkerboard(128));
    result.push_back(not_overlap(128));
    result.push_back(lines_net(128));
    result.push_back(windows(128));
    result.push_back(nested_squares(4096));
    return result;
}

[[nodiscard]] PolygonSet make_set(const std::vector<Rectangle>& rectangles) {
    PolygonSet result{bp::HORIZONTAL};
    result.insert(rectangles.begin(), rectangles.end());
    return result;
}

[[nodiscard]] PolygonSet apply_operation(
    const PolygonSet& subject,
    const PolygonSet& clip,
    Operation operation
) {
    switch (operation) {
        case Operation::xor_:
            return subject ^ clip;
        case Operation::union_:
            return subject | clip;
        case Operation::intersection:
            return subject & clip;
        case Operation::difference:
            return subject - clip;
    }
    std::abort();
}

[[nodiscard]] Polygons materialize(
    const PolygonSet& subject,
    const PolygonSet& clip,
    Operation operation
) {
    auto result = apply_operation(subject, clip, operation);
    Polygons polygons;
    result.get(polygons);
    return polygons;
}

[[nodiscard]] Polygons solve_end_to_end(const Workload& workload) {
    const auto subject = make_set(workload.subject);
    const auto clip = make_set(workload.clip);
    return materialize(subject, clip, workload.operation);
}

void consume(const Polygons& polygons) {
    output_sink = output_sink ^ polygons.size();
}

template <typename OperationFn>
[[nodiscard]] Measurement measure_for(std::chrono::milliseconds budget, OperationFn operation) {
    consume(operation());

    const auto start = std::chrono::steady_clock::now();
    std::size_t iterations = 0;
    do {
        consume(operation());
        ++iterations;
    } while (std::chrono::steady_clock::now() - start < budget);

    return {
        .iterations = iterations,
        .elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
            std::chrono::steady_clock::now() - start
        ),
    };
}

void print_measurement(std::string_view name, const Measurement& measurement) {
    const auto milliseconds = measurement.milliseconds_per_iteration();
    std::cout << "  " << std::left << std::setw(24) << name << std::right << std::fixed
              << std::setprecision(3) << std::setw(10) << milliseconds << " ms/iter, "
              << std::setw(10) << std::setprecision(1) << 1000.0 / milliseconds
              << " iter/s, iterations=" << measurement.iterations << '\n';
}

void run_workload(const Workload& workload, std::chrono::milliseconds budget) {
    const auto prepared_subject = make_set(workload.subject);
    const auto prepared_clip = make_set(workload.clip);
    if (prepared_subject.orient() != bp::HORIZONTAL || prepared_clip.orient() != bp::HORIZONTAL) {
        throw std::runtime_error("polygon_90_set_data is not using HORIZONTAL scan orientation");
    }

    auto validation = apply_operation(prepared_subject, prepared_clip, workload.operation);
    const auto actual_area = bp::area(validation);
    if (actual_area != workload.expected_area) {
        throw std::runtime_error(
            workload.name + " area mismatch: expected " +
            std::to_string(workload.expected_area) + ", got " + std::to_string(actual_area)
        );
    }
    const auto validation_output = materialize(
        prepared_subject,
        prepared_clip,
        workload.operation
    );
    if (validation_output.empty()) {
        throw std::runtime_error(workload.name + " unexpectedly produced no polygons");
    }

    const auto end_to_end = measure_for(budget, [&] {
        return solve_end_to_end(workload);
    });
    const auto prepared = measure_for(budget, [&] {
        return materialize(prepared_subject, prepared_clip, workload.operation);
    });

    std::cout << workload.name << ": n=" << workload.n
              << ", inputs=" << workload.subject.size() + workload.clip.size()
              << ", area=" << actual_area
              << ", output_components=" << validation_output.size() << '\n';
    print_measurement("Boost 90 end-to-end", end_to_end);
    print_measurement("Boost 90 prepared", prepared);
}

int main(int argc, char** argv) try {
    const auto config = parse_config(argc, argv);
    const auto workloads = make_workloads(config);

    std::cout << "Boost.Polygon " << BOOST_LIB_VERSION << '\n'
              << "solver=polygon_90_set_data<int32_t>, "
                 "geometry_concept=polygon_90_set_concept, scan=HORIZONTAL, "
                 "output=polygon_90_with_holes_data<int32_t>, budget="
              << config.budget.count() << "ms\n";
    for (const auto& workload : workloads) {
        run_workload(workload, config.budget);
    }
    return EXIT_SUCCESS;
} catch (const std::exception& error) {
    std::cerr << "error: " << error.what() << '\n';
    return EXIT_FAILURE;
}
