from __future__ import annotations

import math
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Optional, Sequence

from manim import (
    AnimationGroup,
    Arrow,
    CapStyleType,
    Create,
    DashedLine,
    Dot,
    FadeIn,
    FadeOut,
    GrowArrow,
    Indicate,
    LaggedStart,
    Line,
    LineJointType,
    MoveAlongPath,
    Polygon,
    Scene,
    Square,
    Text,
    Transform,
    UpdateFromAlphaFunc,
    VGroup,
    VMobject,
    config,
    linear,
)


# The source artwork is a 200 x 100 SVG made on a 10-unit grid.
SVG_WIDTH = 200
SVG_HEIGHT = 100
CELL = 10
GRID_COLUMNS = SVG_WIDTH // CELL
GRID_ROWS = SVG_HEIGHT // CELL
COLUMN_BOUNDS = (0, 5, 10, 15, 20)

# Change this value to 0, 1, 2, or 3. The sweep is generated from geometry.
ACTIVE_COLUMN = 1

# 200 source units become 12.8 Manim units. In a 16:9 frame this leaves
# 0.71 units at each side and 0.8 units above/below the full SVG viewBox.
SCENE_SCALE = 12.8 / SVG_WIDTH

# Light warm palette. The two SVG source colors stay separate below so the
# artwork can be parsed independently from its presentation colors.
BACKGROUND = "#FFFFFF"
TITLE_COLOR = "#392B25"
SUBJECT_COLOR = "#E9785B"
CLIP_COLOR = "#DCA12D"
SPLIT_COLOR = "#C95643"
INACTIVE_COLOR = "#B9ACA3"
ACTIVE_COLOR = "#D83B32"
PENDING_COLOR = "#D98D1B"
PROCESSED_COLOR = "#718B3C"
RESULT_COLOR = "#3C2D27"
RESULT_FILL = "#EE9B72"
EXTRACT_COLOR = "#168C7A"
HOLE_COLOR = "#A9527B"
TEXT_COLOR = "#6B554A"
GUIDE_COLOR = "#9D8B81"
BUILT_EDGE_COLOR = "#8C8C8C"
FONT_FAMILY = "Menlo"
SVG_SUBJECT_COLOR = "#5599FF"
SVG_CLIP_COLOR = "#DE87CD"
ALGORITHM_TIME_SCALE = 2.0

config.background_color = BACKGROUND

Point = tuple[int, int]
Edge = tuple[Point, Point]
Count = tuple[int, int]


@dataclass(frozen=True)
class InputSegment:
    y: int
    x_min: int
    x_max: int
    count: Count


@dataclass(frozen=True)
class ScanAnchor:
    x: int
    node: Optional[int]
    left: Count
    right: Count


@dataclass(frozen=True)
class StackPoint:
    x: int
    node: Optional[int]
    c0: Count
    c1: Count
    cb: Count


@dataclass(frozen=True)
class ColumnScanEvent:
    point: Point
    node_index: Optional[int]
    is_graph_node: bool
    down_placeholder: Optional[int]
    up_placeholder: Optional[int]
    horizontal_from: Optional[Point]
    opens_horizontal: bool


@dataclass(frozen=True)
class ColumnScanLine:
    y: int
    events: tuple[ColumnScanEvent, ...]


@dataclass(frozen=True)
class GraphLinkPlan:
    node: int
    fill: int


@dataclass(frozen=True)
class GraphNodePlan:
    point: Point
    links: tuple[
        Optional[GraphLinkPlan],
        Optional[GraphLinkPlan],
        Optional[GraphLinkPlan],
        Optional[GraphLinkPlan],
    ]


@dataclass(frozen=True)
class ColumnGraphPlan:
    scan_lines: tuple[ColumnScanLine, ...]
    nodes: tuple[GraphNodePlan, ...]


@dataclass(frozen=True)
class ExtractContourPlan:
    node_indices: tuple[int, ...]
    raw_points: tuple[Point, ...]
    contour: tuple[Point, ...]
    is_hull: bool


@dataclass(frozen=True)
class BorderArcPlan:
    contour_index: int
    out_position: int
    in_position: int
    side: int
    points: tuple[Point, ...]
    next_arc: int


@dataclass(frozen=True)
class BorderEventPlan:
    point: Point
    arc_index: int
    side: int
    is_out: bool


@dataclass(frozen=True)
class BorderMergePlan:
    border_x: int
    arcs: tuple[BorderArcPlan, ...]
    events: tuple[BorderEventPlan, ...]
    event_pairs: tuple[tuple[BorderEventPlan, BorderEventPlan], ...]
    cycles: tuple[tuple[int, ...], ...]
    contours: tuple[tuple[Point, ...], ...]
    untouched: tuple[tuple[Point, ...], ...]


@dataclass
class _BuildNode:
    point: Optional[Point]
    links: list[Optional[GraphLinkPlan]]


@dataclass(frozen=True)
class ShapeSpec:
    operand: str
    color: str
    vertices: tuple[tuple[float, float], ...]


def _style_dict(raw: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for item in raw.split(";"):
        if ":" in item:
            key, value = item.split(":", 1)
            result[key.strip()] = value.strip()
    return result


def _parse_orthogonal_path(path_data: str) -> tuple[tuple[float, float], ...]:
    """Parse the M/L/H/V subset used by tetris.svg."""
    tokens = re.findall(r"[MmLlHhVvZz]|[-+]?(?:\d*\.\d+|\d+)", path_data)
    vertices: list[tuple[float, float]] = []
    x = y = 0.0
    command = ""
    index = 0

    while index < len(tokens):
        token = tokens[index]
        if token.isalpha():
            command = token
            index += 1
            if command in "Zz":
                continue

        if command in "MmLl":
            nx, ny = float(tokens[index]), float(tokens[index + 1])
            index += 2
            if command.islower():
                nx += x
                ny += y
            x, y = nx, ny
            vertices.append((x, y))
            if command == "M":
                command = "L"
            elif command == "m":
                command = "l"
        elif command in "Hh":
            nx = float(tokens[index])
            index += 1
            x = x + nx if command == "h" else nx
            vertices.append((x, y))
        elif command in "Vv":
            ny = float(tokens[index])
            index += 1
            y = y + ny if command == "v" else ny
            vertices.append((x, y))
        else:
            raise ValueError(f"Unsupported SVG command in: {path_data}")

    if len(vertices) > 1 and vertices[0] == vertices[-1]:
        vertices.pop()
    return tuple(vertices)


def load_shape_specs(svg_path: Path) -> list[ShapeSpec]:
    root = ET.parse(svg_path).getroot()
    fill_to_operand = {
        SVG_SUBJECT_COLOR.lower(): "subject",
        SVG_CLIP_COLOR.lower(): "clip",
    }
    operand_color = {
        "subject": SUBJECT_COLOR,
        "clip": CLIP_COLOR,
    }
    result: list[ShapeSpec] = []

    for element in root.iter():
        if not element.tag.endswith("path"):
            continue
        style = _style_dict(element.attrib.get("style", ""))
        fill = style.get("fill", element.attrib.get("fill", "")).lower()
        operand = fill_to_operand.get(fill)
        if operand is None:
            continue  # Ignores the three red guide paths in the source SVG.
        vertices = _parse_orthogonal_path(element.attrib["d"])
        result.append(ShapeSpec(operand, operand_color[operand], vertices))

    if not result:
        raise ValueError(f"No subject/clip paths found in {svg_path}")
    return result


def svg_to_scene(point: tuple[float, float]) -> tuple[float, float, float]:
    x, y = point
    return (
        (x - SVG_WIDTH / 2) * SCENE_SCALE,
        (SVG_HEIGHT / 2 - y) * SCENE_SCALE,
        0.0,
    )


def grid_to_scene(point: Point) -> tuple[float, float, float]:
    x, y = point
    return (
        (x * CELL - SVG_WIDTH / 2) * SCENE_SCALE,
        (y * CELL - SVG_HEIGHT / 2) * SCENE_SCALE,
        0.0,
    )


def _signed_area(points: Sequence[tuple[float, float]]) -> float:
    return 0.5 * sum(
        x0 * y1 - x1 * y0
        for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1])
    )


def _ccw_scene_vertices(spec: ShapeSpec) -> list[tuple[float, float]]:
    points = [(x, SVG_HEIGHT - y) for x, y in spec.vertices]
    if _signed_area(points) < 0:
        points.reverse()
    return points


def _point_inside(point: tuple[float, float], polygon: Sequence[tuple[float, float]]) -> bool:
    px, py = point
    inside = False
    for (x0, y0), (x1, y1) in zip(polygon, polygon[1:] + polygon[:1]):
        if (y0 > py) != (y1 > py):
            crossing_x = (x1 - x0) * (py - y0) / (y1 - y0) + x0
            if px < crossing_x:
                inside = not inside
    return inside


def rasterize(specs: Sequence[ShapeSpec]) -> tuple[set[Point], set[Point]]:
    subject: set[Point] = set()
    clip: set[Point] = set()
    for spec in specs:
        target = subject if spec.operand == "subject" else clip
        for x in range(GRID_COLUMNS):
            for y in range(GRID_ROWS):
                # Convert the y-up grid cell center back to SVG's y-down space.
                center = (x * CELL + CELL / 2, SVG_HEIGHT - (y * CELL + CELL / 2))
                if _point_inside(center, spec.vertices):
                    target.add((x, y))
    return subject, clip


def _add_count(a: Count, b: Count) -> Count:
    return (a[0] + b[0], a[1] + b[1])


def _is_empty_count(count: Count) -> bool:
    return count == (0, 0)


def _inside_union(count: Count) -> bool:
    # NonZeroStrategy + UnionFilter: an edge is retained only when exactly one
    # of its two sides is inside either operand.
    return count[0] != 0 or count[1] != 0


def _segment_fill(top: Count, bottom: Count) -> int:
    return (
        int(top[0] != 0)
        | (int(bottom[0] != 0) << 1)
        | (int(top[1] != 0) << 2)
        | (int(bottom[1] != 0) << 3)
    )


def _is_union_fill(fill: int) -> bool:
    top = fill & 0b0101
    bottom = fill & 0b1010
    return (top == 0 or bottom == 0) and fill != 0


def _union_link(a: Count, b: Count) -> bool:
    return _is_union_fill(_segment_fill(a, b))


def _partition_line(segments: Sequence[InputSegment]) -> list[InputSegment]:
    """Equivalent to partition::LineSolver for one horizontal line."""
    if not segments:
        return []
    xs = sorted({segment.x_min for segment in segments} | {segment.x_max for segment in segments})
    result: list[InputSegment] = []
    y = segments[0].y
    for x0, x1 in zip(xs, xs[1:]):
        count = (0, 0)
        for segment in segments:
            if segment.x_min <= x0 and x1 <= segment.x_max:
                count = _add_count(count, segment.count)
        if _is_empty_count(count):
            continue
        if result and result[-1].x_max == x0 and result[-1].count == count:
            previous = result[-1]
            result[-1] = InputSegment(y, previous.x_min, x1, count)
        else:
            result.append(InputSegment(y, x0, x1, count))
    return result


def column_input_segments(specs: Sequence[ShapeSpec], column: int) -> list[InputSegment]:
    """Build the same directed, clipped horizontal segments as ColumnMap."""
    x_start = COLUMN_BOUNDS[column]
    x_end = COLUMN_BOUNDS[column + 1]
    by_y: dict[int, list[InputSegment]] = {}

    for spec in specs:
        points = _ccw_scene_vertices(spec)
        for a, b in zip(points, points[1:] + points[:1]):
            if a[1] != b[1]:
                continue
            ax, ay = round(a[0] / CELL), round(a[1] / CELL)
            bx = round(b[0] / CELL)
            low = max(x_start, min(ax, bx))
            high = min(x_end, max(ax, bx))
            if low >= high:
                continue
            direction = 1 if ax < bx else -1
            count = (direction, 0) if spec.operand == "subject" else (0, direction)
            by_y.setdefault(ay, []).append(InputSegment(ay, low, high, count))

    result: list[InputSegment] = []
    for y in sorted(by_y):
        result.extend(_partition_line(by_y[y]))
    return result


def _segment_split_points(segments: Sequence[InputSegment]) -> list[tuple[int, Count, Count]]:
    xs = sorted({segment.x_min for segment in segments} | {segment.x_max for segment in segments})
    result = []
    for x in xs:
        left = next(
            (segment.count for segment in segments if segment.x_min < x <= segment.x_max),
            (0, 0),
        )
        right = next(
            (segment.count for segment in segments if segment.x_min <= x < segment.x_max),
            (0, 0),
        )
        result.append((x, left, right))
    return result


def _stack_points(segments: Sequence[InputSegment], anchors: Sequence[ScanAnchor]) -> list[StackPoint]:
    splits = _segment_split_points(segments)
    result: list[StackPoint] = []
    split_index = anchor_index = 0

    while split_index < len(splits) or anchor_index < len(anchors):
        split = splits[split_index] if split_index < len(splits) else None
        anchor = anchors[anchor_index] if anchor_index < len(anchors) else None

        if split is not None and anchor is not None and split[0] == anchor.x:
            result.append(
                StackPoint(
                    split[0],
                    anchor.node,
                    _add_count(split[1], anchor.left),
                    _add_count(split[2], anchor.right),
                    anchor.right,
                )
            )
            split_index += 1
            anchor_index += 1
        elif split is not None and (anchor is None or split[0] < anchor.x):
            if anchor is None:
                result.append(StackPoint(split[0], None, split[1], split[2], (0, 0)))
            else:
                # StackIter::Ordering::Less: the next anchor's left count is
                # the active baseline across this entire x interval.
                result.append(
                    StackPoint(
                        split[0],
                        None,
                        _add_count(split[1], anchor.left),
                        _add_count(split[2], anchor.left),
                        anchor.left,
                    )
                )
            split_index += 1
        else:
            assert anchor is not None
            split_left = split[1] if split is not None else (0, 0)
            result.append(
                StackPoint(
                    anchor.x,
                    anchor.node,
                    _add_count(split_left, anchor.left),
                    _add_count(split_left, anchor.right),
                    anchor.right,
                )
            )
            anchor_index += 1
    return result


def _push_and_merge(anchors: list[ScanAnchor], anchor: ScanAnchor) -> None:
    if anchors and anchors[-1].left == anchor.left:
        anchors[-1] = anchor
    else:
        anchors.append(anchor)


def build_column_graph_plan(specs: Sequence[ShapeSpec], column: int) -> ColumnGraphPlan:
    """Model ColumnGraph::with_fill_and_filter_strategy event for event.

    This intentionally mirrors ScanBuffer::append_changed_topology: rows are
    processed bottom-to-top, StackIter positions left-to-right, and an up node
    remains a point-less placeholder until a later row supplies its position.
    """
    input_segments = column_input_segments(specs, column)
    anchors: list[ScanAnchor] = []
    nodes: list[_BuildNode] = []
    lines: list[ColumnScanLine] = []

    index = 0
    while index < len(input_segments):
        start = index
        y = input_segments[index].y
        while index < len(input_segments) and input_segments[index].y == y:
            index += 1
        segments = input_segments[start:index]
        first_x = segments[0].x_min
        last_x = segments[-1].x_max
        active_start = next((i for i, anchor in enumerate(anchors) if anchor.x >= first_x), len(anchors))
        active_end = next((i for i, anchor in enumerate(anchors) if anchor.x > last_x), len(anchors))

        # Every tetris scanline has fewer than IN_PLACE_SEGMENTS_LIMIT (16)
        # segments, so reproduce add_segments_in_place rather than the buffered
        # branch. Its splice boundaries are important: they preserve unchanged
        # placeholder nodes immediately outside the changed x range.
        buffer: list[ScanAnchor] = []
        left_cursor: Optional[tuple[int, int]] = None
        events: list[ColumnScanEvent] = []

        for stack in _stack_points(segments, anchors[active_start:]):
            if stack.x > last_x:
                break

            down_link = stack.node is not None
            up_link = _union_link(stack.c0, stack.c1)
            down_placeholder = stack.node
            up_placeholder: Optional[int] = None
            horizontal_from: Optional[Point] = None
            opens_horizontal = False
            node_index: Optional[int] = None

            if down_link or up_link:
                point = (stack.x, y)
                up_fill = _segment_fill(stack.c0, stack.c1)
                if down_link:
                    assert stack.node is not None
                    node_index = stack.node
                    nodes[node_index].point = point
                    if up_link:
                        up_placeholder = len(nodes)
                        nodes[node_index].links[1] = GraphLinkPlan(up_placeholder, up_fill)
                        top = _BuildNode(None, [None, None, None, None])
                        top.links[3] = GraphLinkPlan(node_index, up_fill)
                        nodes.append(top)
                else:
                    # Rust creates both `this` and the point-less `up` node.
                    node_index = len(nodes)
                    up_placeholder = node_index + 1
                    node = _BuildNode(point, [None, None, None, None])
                    node.links[1] = GraphLinkPlan(up_placeholder, up_fill)
                    top = _BuildNode(None, [None, None, None, None])
                    top.links[3] = GraphLinkPlan(node_index, up_fill)
                    nodes.extend((node, top))

                if left_cursor is not None:
                    left_node, left_fill = left_cursor
                    assert node_index is not None
                    horizontal_from = nodes[left_node].point
                    assert horizontal_from is not None
                    nodes[left_node].links[2] = GraphLinkPlan(node_index, left_fill)
                    nodes[node_index].links[0] = GraphLinkPlan(left_node, left_fill)
                    left_cursor = None

                right_fill = _segment_fill(stack.c1, stack.cb)
                if _is_union_fill(right_fill):
                    assert node_index is not None
                    left_cursor = (node_index, right_fill)
                    opens_horizontal = True

                _push_and_merge(
                    buffer,
                    ScanAnchor(stack.x, up_placeholder, stack.c0, stack.c1),
                )

            else:
                _push_and_merge(buffer, ScanAnchor(stack.x, None, stack.c0, stack.c1))

            events.append(
                ColumnScanEvent(
                    (stack.x, y),
                    node_index,
                    down_link or up_link,
                    down_placeholder,
                    up_placeholder,
                    horizontal_from,
                    opens_horizontal,
                )
            )

        replace_start = active_start
        if (
            replace_start > 0
            and buffer
            and buffer[0].left == anchors[replace_start - 1].left
        ):
            replace_start -= 1

        suffix = anchors[active_end] if active_end < len(anchors) else None
        if suffix is not None:
            if buffer and buffer[-1].left == suffix.left:
                buffer.pop()
            elif (
                not buffer
                and replace_start == active_start
                and replace_start > 0
                and anchors[replace_start - 1].left == suffix.left
            ):
                replace_start -= 1

        anchors[replace_start:active_end] = buffer
        if len(anchors) == 1 and _is_empty_count(anchors[0].left):
            anchors.clear()
        lines.append(ColumnScanLine(y, tuple(events)))

    frozen_nodes = []
    for node in nodes:
        if node.point is None:
            raise ValueError("ColumnGraph contains an unresolved point-less node")
        frozen_nodes.append(
            GraphNodePlan(
                node.point,
                (node.links[0], node.links[1], node.links[2], node.links[3]),
            )
        )
    return ColumnGraphPlan(tuple(lines), tuple(frozen_nodes))


def _add_skipping_vertical(points: list[Point], point: Point) -> None:
    if len(points) >= 2 and points[-2][0] == points[-1][0] == point[0]:
        points[-1] = point
    else:
        points.append(point)


def extract_column_graph(graph: ColumnGraphPlan) -> list[ExtractContourPlan]:
    """Mirror ColumnGraph::extract/find_contour for OverlayRule::Union."""
    left, up, right, down = 0, 1, 2, 3
    opposite = (right, down, left, up)
    visited = []
    for node in graph.nodes:
        mask = 0
        for order, link in enumerate(node.links):
            if link is not None:
                mask |= 1 << order
        visited.append(mask)

    result: list[ExtractContourPlan] = []
    for start in range(len(graph.nodes)):
        right_bit = 1 << right
        if visited[start] & right_bit == 0:
            continue
        visited[start] &= ~right_bit

        start_link = graph.nodes[start].links[right]
        assert start_link is not None
        link_index = right
        next_node = start_link.node
        node_indices = [start, next_node]

        while next_node != start:
            incoming = opposite[link_index]
            visited[next_node] &= ~(1 << incoming)
            if visited[next_node] == 0:
                raise ValueError(f"Open contour while extracting node {next_node}")

            next_link_index = None
            # LinkIndex::shift(true): Left -> Up -> Right -> Down -> Left.
            candidate = incoming
            for _ in range(4):
                candidate = (candidate + 1) & 0x03
                bit = 1 << candidate
                if visited[next_node] & bit:
                    visited[next_node] &= ~bit
                    next_link_index = candidate
                    break
            if next_link_index is None:
                raise ValueError(f"No outgoing link at node {next_node}")

            link = graph.nodes[next_node].links[next_link_index]
            assert link is not None
            next_node = link.node
            link_index = next_link_index
            node_indices.append(next_node)

        raw_points = [graph.nodes[index].point for index in node_indices]
        contour_points = [raw_points[0]]
        for point in raw_points[1:-1]:
            _add_skipping_vertical(contour_points, point)
        if (
            len(contour_points) >= 2
            and contour_points[0][0] == contour_points[-1][0] == contour_points[-2][0]
        ):
            contour_points.pop()

        # OverlayRule::Union::is_fill_top => BOTH_BOTTOM == NONE.
        is_hull = start_link.fill & 0b1010 == 0
        if not is_hull and len(contour_points) > 1:
            contour_points[1:] = reversed(contour_points[1:])

        result.append(
            ExtractContourPlan(
                tuple(node_indices),
                tuple(raw_points),
                tuple(contour_points),
                is_hull,
            )
        )
    return result


def _open_contour(contour: Sequence[Point]) -> tuple[Point, ...]:
    points = tuple(contour)
    if len(points) > 1 and points[0] == points[-1]:
        points = points[:-1]
    return points


def _arc_points(contour: Sequence[Point], start: int, end: int) -> tuple[Point, ...]:
    if start <= end:
        return tuple(contour[start : end + 1])
    return tuple(contour[start:]) + tuple(contour[: end + 1])


def _is_collinear(a: Point, b: Point, c: Point) -> bool:
    return (b[0] - a[0]) * (c[1] - b[1]) == (b[1] - a[1]) * (c[0] - b[0])


def _close_merged_contour(contour: list[Point]) -> tuple[Point, ...]:
    while len(contour) > 1 and contour[0] == contour[-1]:
        contour.pop()
    while len(contour) >= 3:
        if _is_collinear(contour[-1], contour[0], contour[1]):
            contour.pop(0)
        elif _is_collinear(contour[-2], contour[-1], contour[0]):
            contour.pop()
        else:
            break
    return tuple(contour)


def build_border_merge_plan(
    left_contours: Sequence[Sequence[Point]],
    right_contours: Sequence[Sequence[Point]],
    border_x: int,
) -> BorderMergePlan:
    """Mirror merge_border_contours, retaining non-border contours for the scene."""
    contours = [_open_contour(contour) for contour in (*left_contours, *right_contours)]
    left_count = len(left_contours)
    raw_arcs: list[dict[str, int | tuple[Point, ...]]] = []
    events: list[BorderEventPlan] = []
    touched: set[int] = set()

    def push_arc(
        contour_index: int,
        side: int,
        out_position: int,
        in_position: int,
    ) -> None:
        arc_index = len(raw_arcs)
        contour = contours[contour_index]
        raw_arcs.append(
            {
                "contour_index": contour_index,
                "out_position": out_position,
                "in_position": in_position,
                "side": side,
                "points": _arc_points(contour, out_position, in_position),
                "next_arc": -1,
            }
        )
        events.append(BorderEventPlan(contour[out_position], arc_index, side, True))
        events.append(BorderEventPlan(contour[in_position], arc_index, side, False))
        touched.add(contour_index)

    for contour_index, contour in enumerate(contours):
        side = 0 if contour_index < left_count else 1
        first_in: Optional[int] = None
        pending_out: Optional[int] = None
        count = len(contour)
        for position, point in enumerate(contour):
            if point[0] != border_x:
                continue
            previous = contour[(position - 1) % count]
            following = contour[(position + 1) % count]
            previous_off_border = previous[1] == point[1] and previous[0] != border_x
            following_off_border = following[1] == point[1] and following[0] != border_x
            if not previous_off_border and not following_off_border:
                continue
            if previous_off_border == following_off_border:
                raise ValueError(f"Invalid border portal at {point}")

            is_out = following[0] != border_x
            if is_out:
                if pending_out is not None:
                    raise ValueError("Contour portals do not alternate")
                pending_out = position
            elif pending_out is not None:
                push_arc(contour_index, side, pending_out, position)
                pending_out = None
            elif first_in is None:
                first_in = position
            else:
                raise ValueError("Contour portals do not alternate")

        if pending_out is not None and first_in is not None:
            push_arc(contour_index, side, pending_out, first_in)
        elif pending_out is not None or first_in is not None:
            raise ValueError("A contour must have paired border portals")

    sorted_events = sorted(events, key=lambda event: (event.point[1], event.side))
    if len(sorted_events) % 2:
        raise ValueError("Border events must form pairs")

    event_pairs: list[tuple[BorderEventPlan, BorderEventPlan]] = []
    for index in range(0, len(sorted_events), 2):
        a, b = sorted_events[index : index + 2]
        if a.is_out == b.is_out:
            raise ValueError(f"Border pair at y={a.point[1]} has no in/out match")
        input_event, out_event = (b, a) if a.is_out else (a, b)
        raw_arcs[input_event.arc_index]["next_arc"] = out_event.arc_index
        event_pairs.append((a, b))

    arcs = tuple(
        BorderArcPlan(
            int(arc["contour_index"]),
            int(arc["out_position"]),
            int(arc["in_position"]),
            int(arc["side"]),
            arc["points"],  # type: ignore[arg-type]
            int(arc["next_arc"]),
        )
        for arc in raw_arcs
    )

    cycles: list[tuple[int, ...]] = []
    visited: set[int] = set()
    for start in range(len(arcs)):
        if start in visited:
            continue
        cycle: list[int] = []
        current = start
        while current not in visited:
            visited.add(current)
            cycle.append(current)
            current = arcs[current].next_arc
            if current < 0:
                raise ValueError("Border arc has no next arc")
        if current != start:
            raise ValueError("Border arc cycle closed at the wrong start")
        cycles.append(tuple(cycle))

    merged_contours: list[tuple[Point, ...]] = []
    for cycle in cycles:
        result: list[Point] = []
        for arc_index in cycle:
            points = list(arcs[arc_index].points)
            if result and result[-1] == points[0]:
                if len(result) < 2 or len(points) < 2:
                    raise ValueError("A merged arc must contain an edge")
                if not _is_collinear(result[-2], result[-1], points[1]):
                    raise ValueError("A portal must join collinear border edges")
                result.pop()
                points = points[1:]
            result.extend(points)
        merged_contours.append(_close_merged_contour(result))

    untouched = tuple(contours[index] for index in range(len(contours)) if index not in touched)
    return BorderMergePlan(
        border_x,
        arcs,
        tuple(sorted_events),
        tuple(event_pairs),
        tuple(cycles),
        tuple(merged_contours) + untouched,
        untouched,
    )


def boundary_edges(cells: set[Point], x_start: int, x_end: int) -> list[Edge]:
    """Return directed edges with occupied space on their left (CCW outer rings)."""
    kept: dict[tuple[Point, Point], Edge] = {}

    def add(edge: Edge) -> None:
        a, b = edge
        key = tuple(sorted((a, b)))
        if key in kept:
            del kept[key]
        else:
            kept[key] = edge

    for x, y in cells:
        if not x_start <= x < x_end:
            continue
        add(((x, y), (x + 1, y)))
        add(((x + 1, y), (x + 1, y + 1)))
        add(((x + 1, y + 1), (x, y + 1)))
        add(((x, y + 1), (x, y)))
    return list(kept.values())


def _turn_priority(incoming: Point, outgoing: Point) -> tuple[int, int]:
    cross = incoming[0] * outgoing[1] - incoming[1] * outgoing[0]
    dot = incoming[0] * outgoing[0] + incoming[1] * outgoing[1]
    if cross > 0:
        return (3, dot)  # left
    if dot > 0:
        return (2, dot)  # straight
    if cross < 0:
        return (1, dot)  # right
    return (0, dot)  # reverse


def _compress_closed_contour(points: list[Point]) -> list[Point]:
    ring = points[:-1]
    compressed: list[Point] = []
    for index, current in enumerate(ring):
        previous = ring[index - 1]
        following = ring[(index + 1) % len(ring)]
        before = (current[0] - previous[0], current[1] - previous[1])
        after = (following[0] - current[0], following[1] - current[1])
        if before[0] * after[1] != before[1] * after[0]:
            compressed.append(current)
    start = min(range(len(compressed)), key=lambda i: (compressed[i][1], compressed[i][0]))
    compressed = compressed[start:] + compressed[:start]
    return compressed + [compressed[0]]


def trace_contours(edges: Iterable[Edge]) -> list[list[Point]]:
    remaining = set(edges)
    outgoing: dict[Point, list[Edge]] = {}
    for edge in remaining:
        outgoing.setdefault(edge[0], []).append(edge)

    contours: list[list[Point]] = []
    while remaining:
        first = min(remaining, key=lambda edge: (edge[0][1], edge[0][0], edge[1]))
        remaining.remove(first)
        points = [first[0], first[1]]
        previous, current = first

        while current != points[0]:
            candidates = [edge for edge in outgoing.get(current, []) if edge in remaining]
            if not candidates:
                raise ValueError(f"Open contour at {current}")
            incoming = (current[0] - previous[0], current[1] - previous[1])
            chosen = max(
                candidates,
                key=lambda edge: _turn_priority(
                    incoming,
                    (edge[1][0] - edge[0][0], edge[1][1] - edge[0][1]),
                ),
            )
            remaining.remove(chosen)
            previous, current = chosen
            points.append(current)

        contours.append(_compress_closed_contour(points))

    contours.sort(key=lambda ring: (min(y for _, y in ring), min(x for x, _ in ring)))
    return contours


def contours_for_span(union_cells: set[Point], x_start: int, x_end: int) -> list[list[Point]]:
    return trace_contours(boundary_edges(union_cells, x_start, x_end))


def split_horizontal_edge(a: tuple[float, float], b: tuple[float, float]) -> list[tuple[tuple[float, float], tuple[float, float], int]]:
    low, high = sorted((a[0], b[0]))
    cuts = [low] + [x for x in (50, 100, 150) if low < x < high] + [high]
    pieces = list(zip(cuts, cuts[1:]))
    if a[0] > b[0]:
        pieces.reverse()

    result = []
    for left, right in pieces:
        start, end = ((left, a[1]), (right, a[1]))
        if a[0] > b[0]:
            start, end = end, start
        column = min(3, int(((left + right) / 2) // 50))
        result.append((start, end, column))
    return result


def arrow_for_segment(start: tuple[float, float], end: tuple[float, float], color: str) -> Arrow:
    scene_start = svg_to_scene((start[0], SVG_HEIGHT - start[1]))
    scene_end = svg_to_scene((end[0], SVG_HEIGHT - end[1]))
    length = math.dist(scene_start[:2], scene_end[:2])
    return Arrow(
        scene_start,
        scene_end,
        buff=0,
        color=color,
        stroke_width=4.5,
        tip_length=min(0.15, length * 0.24),
        max_tip_length_to_length_ratio=0.25,
    )


def contour_paths(contours: Sequence[Sequence[Point]], color: str = RESULT_COLOR) -> VGroup:
    result = VGroup()
    for contour in contours:
        path = VMobject(
            stroke_color=color,
            stroke_width=6,
            fill_opacity=0,
            joint_type=LineJointType.MITER,
            cap_style=CapStyleType.ROUND,
        )
        path.set_points_as_corners([grid_to_scene(point) for point in contour])
        result.add(path)
    return result


def column_extract_paths(
    contours: Sequence[Sequence[Point]],
    x_start: int,
    x_end: int,
) -> VGroup:
    """Style extracted chunks by whether they touch a vertical column border."""
    result = VGroup()
    for contour in contours:
        touches_border = any(point[0] in (x_start, x_end) for point in contour)
        color = EXTRACT_COLOR if touches_border else HOLE_COLOR
        path = VMobject(
            stroke_color=color,
            stroke_width=7,
            fill_opacity=0,
            joint_type=LineJointType.MITER,
            cap_style=CapStyleType.ROUND,
        )
        path.set_points_as_corners([grid_to_scene(point) for point in contour])
        result.add(path)
    return result


class TetrisUnionScene(Scene):
    def construct(self) -> None:
        svg_path = Path(__file__).with_name("tetris.svg")
        specs = load_shape_specs(svg_path)
        subject_cells, clip_cells = rasterize(specs)
        union_cells = subject_cells | clip_cells
        assert 0 <= ACTIVE_COLUMN < 4

        title = Text(
            "UNION",
            font=FONT_FAMILY,
            font_size=30,
            weight="BOLD",
            color=TITLE_COLOR,
        ).move_to((-5.7, 3.58, 0))
        subject_key = VGroup(
            Dot(radius=0.075, color=SUBJECT_COLOR),
            Text("SUBJECT", font=FONT_FAMILY, font_size=18, color=SUBJECT_COLOR),
        ).arrange(buff=0.12)
        clip_key = VGroup(
            Dot(radius=0.075, color=CLIP_COLOR),
            Text("CLIP", font=FONT_FAMILY, font_size=18, color=CLIP_COLOR),
        ).arrange(buff=0.12)
        legend = VGroup(subject_key, clip_key).arrange(buff=0.34).move_to((4.95, 3.58, 0))
        self.phase = Text(
            "subject + clip",
            font=FONT_FAMILY,
            font_size=22,
            color=TEXT_COLOR,
        ).move_to((0, -3.62, 0))
        self.add(title, legend, self.phase)

        shapes = VGroup()
        for spec in specs:
            polygon = Polygon(
                *[svg_to_scene(point) for point in spec.vertices],
                stroke_color=spec.color,
                stroke_width=2.2,
                fill_color=spec.color,
                fill_opacity=0.9,
            )
            shapes.add(polygon)

        # 1. Source composition.
        self.play(LaggedStart(*[FadeIn(shape, scale=0.94) for shape in shapes], lag_ratio=0.055), run_time=1.8)
        self.wait(0.65)

        # 2. Four spatial columns.
        splitters = VGroup(
            *[
                DashedLine(
                    svg_to_scene((x, 4)),
                    svg_to_scene((x, 96)),
                    dash_length=0.16,
                    dashed_ratio=0.55,
                    color=SPLIT_COLOR,
                    stroke_width=3,
                )
                for x in (50, 100, 150)
            ]
        )
        self.set_phase("divide space into four columns")
        self.play(LaggedStart(*[Create(line) for line in splitters], lag_ratio=0.16), run_time=1.25)
        self.wait(0.35)

        # 3. Replace filled polygons by their directed horizontal edges.
        vectors_by_column = [VGroup() for _ in range(4)]
        for spec in specs:
            points = _ccw_scene_vertices(spec)
            for a, b in zip(points, points[1:] + points[:1]):
                if a[1] != b[1]:
                    continue
                for start, end, column in split_horizontal_edge(a, b):
                    vectors_by_column[column].add(arrow_for_segment(start, end, spec.color))
        self.set_phase("horizontal vectors encode the winding rule")
        self.play(
            FadeOut(shapes),
            LaggedStart(*[GrowArrow(arrow) for column in vectors_by_column for arrow in column], lag_ratio=0.018),
            run_time=2.5,
        )
        self.wait(0.55)

        # 4. Isolate one column. Its number is controlled by ACTIVE_COLUMN.
        self.set_phase(f"process column {ACTIVE_COLUMN + 1}")
        dim_animations = []
        for index, column in enumerate(vectors_by_column):
            if index != ACTIVE_COLUMN:
                dim_animations.append(column.animate.set_opacity(0.15).set_color(INACTIVE_COLOR))
        for index, splitter in enumerate(splitters):
            if index not in (ACTIVE_COLUMN - 1, ACTIVE_COLUMN):
                dim_animations.append(splitter.animate.set_opacity(0.22))
        self.play(*dim_animations, run_time=0.8)

        # 5. Bottom-to-top sweep of the selected column.
        active_contours = contours_for_span(
            union_cells,
            COLUMN_BOUNDS[ACTIVE_COLUMN],
            COLUMN_BOUNDS[ACTIVE_COLUMN + 1],
        )
        graph_plan = build_column_graph_plan(specs, ACTIVE_COLUMN)
        built_active = self.animate_column_sweep(
            graph_plan.scan_lines,
            active_contours,
            vectors_by_column[ACTIVE_COLUMN],
        )
        self.wait(0.45 * ALGORITHM_TIME_SCALE)

        # 5.5. ColumnGraph::extract: start only from an unvisited Right link,
        # walk links using shift(true), and simplify vertical middle nodes.
        self.set_phase("extract: first unvisited RIGHT link", ALGORITHM_TIME_SCALE)
        extracted_active = self.animate_graph_extract(graph_plan, built_active)
        self.wait(0.4 * ALGORITHM_TIME_SCALE)

        # 6. Reveal the completed result of every column.
        self.set_phase("all four columns are complete")
        column_results = VGroup(
            *[
                column_extract_paths(
                    contours_for_span(union_cells, COLUMN_BOUNDS[i], COLUMN_BOUNDS[i + 1]),
                    COLUMN_BOUNDS[i],
                    COLUMN_BOUNDS[i + 1],
                )
                for i in range(4)
            ]
        )
        inactive_vectors = VGroup(
            *[
                vectors_by_column[index]
                for index in range(4)
                if index != ACTIVE_COLUMN
            ]
        )
        # Active-column arrows were removed progressively during the sweep.
        # Do not animate their parent group again: FadeOut would temporarily
        # re-add those removed children before fading them a second time.
        self.remove(*vectors_by_column[ACTIVE_COLUMN])
        self.play(
            FadeOut(inactive_vectors),
            FadeOut(built_active),
            FadeOut(extracted_active),
            FadeIn(column_results),
            *[splitter.animate.set_opacity(0.55) for splitter in splitters],
            run_time=1.1,
        )
        self.wait(0.45)

        # 7. merge_border_contours: collect in/out portals on the shared border,
        # sort them by (y, side), pair them, and follow the resulting arc cycles.
        column_contours = [
            tuple(_open_contour(contour) for contour in contours_for_span(
                union_cells, COLUMN_BOUNDS[index], COLUMN_BOUNDS[index + 1]
            ))
            for index in range(4)
        ]
        left_plan = build_border_merge_plan(column_contours[0], column_contours[1], 5)
        right_plan = build_border_merge_plan(column_contours[2], column_contours[3], 15)
        final_plan = build_border_merge_plan(left_plan.contours, right_plan.contours, 10)

        self.set_phase("merge border: columns 1 + 2")
        left_paths = self.animate_border_merge(
            left_plan,
            VGroup(column_results[0], column_results[1]),
            splitters[0],
        )

        self.set_phase("merge border: columns 3 + 4")
        right_paths = self.animate_border_merge(
            right_plan,
            VGroup(column_results[2], column_results[3]),
            splitters[2],
        )

        self.set_phase("merge border: both halves")
        final_paths = self.animate_border_merge(
            final_plan,
            left_paths,
            splitters[1],
            right_paths,
        )

        # 8. Bright final union.
        final_fill = self.cell_fill(union_cells)
        final_fill.set_z_index(-1)
        final_paths.set_z_index(2)
        self.set_phase("union complete")
        self.play(FadeIn(final_fill), run_time=0.8)
        self.play(Indicate(final_paths, color="#FFF8EA", scale_factor=1.018), run_time=1.1)
        self.wait(1.5)

    def set_phase(self, text: str, time_scale: float = 1.0) -> None:
        replacement = Text(
            text,
            font=FONT_FAMILY,
            font_size=22,
            color=TEXT_COLOR,
        ).move_to(self.phase)
        self.play(Transform(self.phase, replacement), run_time=0.35 * time_scale)

    def animate_column_sweep(
        self,
        scan_lines: Sequence[ColumnScanLine],
        contours: Sequence[Sequence[Point]],
        source_vectors: VGroup,
    ) -> VGroup:
        """Animate the event order of ScanBuffer::append_changed_topology."""
        time_scale = ALGORITHM_TIME_SCALE
        vertical_edges = [
            edge
            for contour in contours
            for edge in zip(contour, contour[1:])
            if edge[0][0] == edge[1][0]
        ]

        def is_upward(point: Point) -> bool:
            x, y = point
            candidates = [
                edge
                for edge in vertical_edges
                if edge[0][0] == x
                and min(edge[0][1], edge[1][1]) <= y < max(edge[0][1], edge[1][1])
            ]
            if not candidates:
                return True
            edge = candidates[0]
            return edge[1][1] > edge[0][1]

        built = VGroup()
        consumed_vectors: set[int] = set()

        def advance_vectors_through(y: int, x: Optional[int] = None) -> None:
            scene_y = grid_to_scene((0, y))[1]
            scene_x = grid_to_scene((x, y))[0] if x is not None else math.inf
            animations = []
            for vector in source_vectors:
                if id(vector) in consumed_vectors:
                    continue
                start = vector.get_start()
                end = vector.get_end()
                if abs(vector.get_center()[1] - scene_y) >= 1e-4:
                    continue
                x_min = min(start[0], end[0])
                x_max = max(start[0], end[0])
                if scene_x <= x_min + 1e-4:
                    continue
                if scene_x >= x_max - 1e-4:
                    consumed_vectors.add(id(vector))
                    animations.append(FadeOut(vector))
                    continue

                # A source arrow can cross several graph-node intervals. Trim
                # its already scanned left part instead of keeping the whole
                # arrow visible until its far endpoint is reached.
                directed_right = end[0] > start[0]
                trimmed_start = (scene_x, scene_y, 0) if directed_right else (x_max, scene_y, 0)
                trimmed_end = (x_max, scene_y, 0) if directed_right else (scene_x, scene_y, 0)
                remaining_length = x_max - scene_x
                trimmed = Arrow(
                    trimmed_start,
                    trimmed_end,
                    buff=0,
                    color=vector.get_color(),
                    stroke_width=4.5,
                    tip_length=min(0.15, remaining_length * 0.24),
                    max_tip_length_to_length_ratio=0.25,
                )
                animations.append(Transform(vector, trimmed))

            if not animations:
                return
            self.play(
                *animations,
                run_time=0.16 * time_scale,
            )

        # placeholder -> (visible ray, real lower endpoint, contour direction)
        pending: dict[int, tuple[Arrow, Point, bool]] = {}
        ray_top_y = 9.62

        for scan_line in scan_lines:
            if not scan_line.events:
                advance_vectors_through(scan_line.y)
                continue

            guides = VGroup(
                *[
                    Dot(
                        grid_to_scene(event.point),
                        radius=0.045,
                        color=GUIDE_COLOR,
                        fill_opacity=0.62,
                    )
                    for event in scan_line.events
                ]
            )
            self.play(FadeIn(guides), run_time=0.12 * time_scale)

            first = scan_line.events[0]
            marker = Dot(grid_to_scene(first.point), radius=0.1, color=ACTIVE_COLOR).set_z_index(6)
            self.play(FadeIn(marker, scale=1.45), run_time=0.16 * time_scale)

            open_horizontal = False
            red_trail = VGroup()
            previous = first.point

            for event_index, event in enumerate(scan_line.events):
                loose_piece = None
                if event_index > 0:
                    scan_piece = Line(
                        grid_to_scene(previous),
                        grid_to_scene(event.point),
                        color=ACTIVE_COLOR,
                        stroke_width=7,
                    ).set_z_index(5)
                    if open_horizontal:
                        red_trail.add(scan_piece)
                    else:
                        loose_piece = scan_piece
                    self.play(
                        Create(scan_piece, rate_func=linear),
                        marker.animate.move_to(grid_to_scene(event.point)),
                        run_time=0.19 * time_scale,
                    )
                    advance_vectors_through(scan_line.y, event.point[0])

                node_animations = []

                if event.down_placeholder is not None:
                    ray, origin, upward = pending.pop(event.down_placeholder)
                    if upward:
                        start, end = origin, event.point
                    else:
                        start, end = event.point, origin
                    final_vertical = Line(
                        grid_to_scene(start),
                        grid_to_scene(end),
                        color=BUILT_EDGE_COLOR,
                        stroke_width=5,
                        cap_style=CapStyleType.ROUND,
                    )
                    node_animations.append(Transform(ray, final_vertical))
                    built.add(ray)

                if event.up_placeholder is not None:
                    upward = is_upward(event.point)
                    top = (event.point[0], ray_top_y)
                    start, end = (event.point, top) if upward else (top, event.point)
                    ray = Arrow(
                        grid_to_scene(start),
                        grid_to_scene(end),
                        buff=0,
                        color=PENDING_COLOR,
                        stroke_width=4.5,
                        tip_length=0.15,
                        max_tip_length_to_length_ratio=0.16,
                    ).set_z_index(2)
                    pending[event.up_placeholder] = (ray, event.point, upward)
                    node_animations.append(GrowArrow(ray))

                if node_animations:
                    self.play(*node_animations, run_time=0.24 * time_scale)

                if event.horizontal_from is not None:
                    horizontal = Line(
                        grid_to_scene(event.horizontal_from),
                        grid_to_scene(event.point),
                        color=BUILT_EDGE_COLOR,
                        stroke_width=5,
                        cap_style=CapStyleType.ROUND,
                    )
                    self.play(
                        FadeOut(red_trail),
                        Create(horizontal),
                        run_time=0.24 * time_scale,
                    )
                    built.add(horizontal)
                    red_trail = VGroup()
                    open_horizontal = False

                if loose_piece is not None:
                    self.play(FadeOut(loose_piece), run_time=0.07 * time_scale)

                if event.opens_horizontal:
                    open_horizontal = True
                previous = event.point

            if len(red_trail) > 0:
                self.play(FadeOut(red_trail), run_time=0.08 * time_scale)
            self.play(
                FadeOut(marker),
                FadeOut(guides),
                run_time=0.13 * time_scale,
            )
            advance_vectors_through(scan_line.y)

        if pending:
            raise ValueError(f"Unresolved ColumnGraph placeholders: {sorted(pending)}")
        return built

    def animate_graph_extract(
        self,
        graph: ColumnGraphPlan,
        graph_mobjects: VGroup,
    ) -> VGroup:
        time_scale = ALGORITHM_TIME_SCALE
        contours = extract_column_graph(graph)
        extracted = VGroup()
        self.play(graph_mobjects.animate.set_opacity(0.24), run_time=0.35 * time_scale)

        for contour_index, contour in enumerate(contours):
            if contour_index > 0:
                self.set_phase("extract: next unvisited RIGHT link", time_scale)

            start = contour.raw_points[0]
            marker = Dot(grid_to_scene(start), radius=0.085, color=ACTIVE_COLOR).set_z_index(8)
            start_ring = (
                Dot(grid_to_scene(start), radius=0.145, color=ACTIVE_COLOR, fill_opacity=0)
                .set_stroke(ACTIVE_COLOR, width=3)
                .set_z_index(7)
            )
            self.play(FadeIn(marker), FadeIn(start_ring), run_time=0.22 * time_scale)

            raw_edges = VGroup()
            x_start = COLUMN_BOUNDS[ACTIVE_COLUMN]
            x_end = COLUMN_BOUNDS[ACTIVE_COLUMN + 1]
            touches_border = any(point[0] in (x_start, x_end) for point in contour.raw_points)
            trace_color = EXTRACT_COLOR if touches_border else HOLE_COLOR
            for edge_index, (a, b) in enumerate(zip(contour.raw_points, contour.raw_points[1:])):
                edge = Line(
                    grid_to_scene(a),
                    grid_to_scene(b),
                    color=trace_color,
                    stroke_width=7,
                    cap_style=CapStyleType.ROUND,
                ).set_z_index(6)
                animations = [
                    Create(edge, rate_func=linear),
                    marker.animate.move_to(grid_to_scene(b)),
                ]
                if edge_index == 0:
                    animations.append(FadeOut(start_ring))
                self.play(*animations, run_time=0.16 * time_scale)
                raw_edges.add(edge)

            # Replace the temporary edge pieces without animation by one
            # continuous miter-joined path. The geometry does not change, but
            # corners no longer expose gaps between separate Line caps.
            continuous_path = VMobject(
                stroke_color=trace_color,
                stroke_width=7,
                fill_opacity=0,
                joint_type=LineJointType.MITER,
                cap_style=CapStyleType.ROUND,
            ).set_z_index(6)
            continuous_path.set_points_as_corners(
                [grid_to_scene(point) for point in contour.raw_points]
            )
            self.remove(*raw_edges)
            self.add(continuous_path)
            self.play(FadeOut(marker), run_time=0.12 * time_scale)
            extracted.add(continuous_path)

        self.play(graph_mobjects.animate.set_opacity(0.09), run_time=0.3 * time_scale)
        return extracted

    def animate_border_merge(
        self,
        plan: BorderMergePlan,
        left_display: VGroup,
        seam: DashedLine,
        right_display: Optional[VGroup] = None,
    ) -> VGroup:
        """Visualize merge_border_contours without replacing it by a final redraw."""
        if right_display is None:
            left_group = VGroup(left_display[0])
            right_group = VGroup(left_display[1])
        else:
            left_group = left_display
            right_group = right_display

        self.play(
            left_group.animate.set_stroke(opacity=0.2),
            right_group.animate.set_stroke(opacity=0.2),
            seam.animate.set_opacity(1).set_color(ACTIVE_COLOR),
            run_time=0.35,
        )

        symbols: dict[BorderEventPlan, VGroup] = {}
        portal_group = VGroup()
        for event in plan.events:
            base = grid_to_scene(event.point)
            direction = -1 if event.side == 0 else 1
            near = (base[0] + 0.065 * direction, base[1], 0)
            far = (base[0] + 0.34 * direction, base[1], 0)
            start, end = (near, far) if event.is_out else (far, near)
            color = EXTRACT_COLOR if event.side == 0 else HOLE_COLOR
            symbol = VGroup(
                Dot(near, radius=0.055, color=color).set_z_index(9),
                Arrow(
                    start,
                    end,
                    buff=0.025,
                    color=color,
                    stroke_width=4,
                    tip_length=0.09,
                ).set_z_index(8),
            )
            symbols[event] = symbol
            portal_group.add(symbol)

        if len(portal_group) > 0:
            self.play(
                LaggedStart(*[FadeIn(symbol) for symbol in portal_group], lag_ratio=0.055),
                run_time=0.7,
            )

        pair_links = VGroup()
        for a, b in plan.event_pairs:
            start = grid_to_scene(a.point)
            end = grid_to_scene(b.point)
            if a.point == b.point:
                link = Dot(start, radius=0.09, color=PROCESSED_COLOR, fill_opacity=0.3)
                animation = FadeIn(link, scale=1.4)
            else:
                link = Line(start, end, color=PROCESSED_COLOR, stroke_width=7)
                animation = Create(link)
            link.set_z_index(7)
            pair_links.add(link)
            self.play(
                Indicate(symbols[a][0], color=PROCESSED_COLOR, scale_factor=1.7),
                Indicate(symbols[b][0], color=PROCESSED_COLOR, scale_factor=1.7),
                animation,
                run_time=0.32,
            )

        outgoing_symbol = {
            event.arc_index: symbols[event]
            for event in plan.events
            if event.is_out
        }
        traced_cycles = VGroup()
        for cycle in plan.cycles:
            traced_arc_group = VGroup()
            first_arc = plan.arcs[cycle[0]]
            marker = Dot(
                grid_to_scene(first_arc.points[0]),
                radius=0.075,
                color=ACTIVE_COLOR,
            ).set_z_index(11)
            self.play(FadeIn(marker), run_time=0.12)
            for arc_index in cycle:
                arc = plan.arcs[arc_index]
                color = EXTRACT_COLOR if arc.side == 0 else HOLE_COLOR
                path = VMobject(
                    stroke_color=color,
                    stroke_width=7,
                    fill_opacity=0,
                    joint_type=LineJointType.MITER,
                    cap_style=CapStyleType.ROUND,
                ).set_z_index(6)
                path.set_points_as_corners([grid_to_scene(point) for point in arc.points])
                distance = sum(
                    abs(a[0] - b[0]) + abs(a[1] - b[1])
                    for a, b in zip(arc.points, arc.points[1:])
                )
                self.play(
                    self.create_path_with_front_marker(path, marker),
                    run_time=max(0.34, min(0.78, distance * 0.065)),
                )
                traced_arc_group.add(path)
                next_arc = plan.arcs[arc.next_arc]
                if arc.points[-1] != next_arc.points[0]:
                    # append_contour_arc stores these two portals as adjacent
                    # contour points. Render their implicit border edge too;
                    # otherwise fading the source columns leaves an open ring.
                    connector = Line(
                        grid_to_scene(arc.points[-1]),
                        grid_to_scene(next_arc.points[0]),
                        color=PROCESSED_COLOR,
                        stroke_width=7,
                        cap_style=CapStyleType.ROUND,
                    ).set_z_index(6)
                    self.play(
                        self.create_path_with_front_marker(connector, marker),
                        run_time=0.16,
                    )
                    traced_arc_group.add(connector)
                next_symbol = outgoing_symbol.get(arc.next_arc)
                if next_symbol is not None:
                    self.play(
                        Indicate(next_symbol[0], color=PROCESSED_COLOR, scale_factor=1.65),
                        run_time=0.16,
                    )
            self.play(
                FadeOut(marker),
                traced_arc_group.animate.set_color(RESULT_COLOR),
                run_time=0.28,
            )
            traced_cycles.add(traced_arc_group)

        untouched_paths = contour_paths(
            [tuple(contour) + (contour[0],) for contour in plan.untouched],
            color=RESULT_COLOR,
        )
        cleanup = [
            FadeOut(left_group),
            FadeOut(right_group),
            FadeOut(portal_group),
            FadeOut(pair_links),
            FadeOut(seam),
        ]
        if len(untouched_paths) > 0:
            cleanup.append(FadeIn(untouched_paths))
        self.play(*cleanup, run_time=0.4)
        return VGroup(traced_cycles, untouched_paths)

    def create_path_with_front_marker(
        self,
        path: VMobject,
        marker: Dot,
    ) -> AnimationGroup:
        """Keep the marker on Create's actual partial-path endpoint."""
        full_path = path.copy()
        partial_path = path.copy()

        def follow_partial_path(dot: Dot, alpha: float) -> None:
            partial_path.pointwise_become_partial(full_path, 0, alpha)
            dot.move_to(partial_path.get_end())

        return AnimationGroup(
            Create(path, rate_func=linear),
            UpdateFromAlphaFunc(marker, follow_partial_path, rate_func=linear),
            lag_ratio=0,
        )

    def trace_paths(self, paths: VGroup, run_time_per_path: float = 0.8) -> None:
        for path in paths:
            marker = Dot(path.get_start(), radius=0.075, color=ACTIVE_COLOR).set_z_index(5)
            self.add(marker)
            self.play(
                AnimationGroup(
                    Create(path, rate_func=linear),
                    MoveAlongPath(marker, path, rate_func=linear),
                    lag_ratio=0,
                ),
                run_time=run_time_per_path,
            )
            self.play(FadeOut(marker), run_time=0.12)

    def cell_fill(self, cells: set[Point]) -> VGroup:
        side = CELL * SCENE_SCALE * 1.006
        result = VGroup()
        for x, y in cells:
            square = Square(
                side_length=side,
                stroke_width=0,
                fill_color=RESULT_FILL,
                fill_opacity=0.42,
            )
            square.move_to(grid_to_scene((x, y)))
            square.shift((side / 2, side / 2, 0))
            result.add(square)
        return result
