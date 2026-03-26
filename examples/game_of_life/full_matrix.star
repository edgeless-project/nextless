inputs = [
    [5, 4, 3, 2, 1],
    [10, 9, 8, 7, 6],
    [15, 14, 13, 12, 11]
]

instances = [
    {
        "id": inputs[row][col],
        "left": inputs[row][col-1] if col > 0 else None,
        "right": inputs[row][col+1] if col < (len(inputs[0]) - 1) else None,
        "bottom": inputs[row+1][col] if row < (len(inputs) - 1) else None,
        "top": inputs[row-1][col] if row > 0 else None,
        "top_left": inputs[row-1][col-1] if row > 0 and col > 0 else None,
        "top_right": inputs[row-1][col+1] if row > 0 and col < (len(inputs[0]) - 1) else None,
        "bottom_left": inputs[row+1][col-1] if row < (len(inputs) - 1) and col > 0 else None,
        "bottom_right": inputs[row+1][col+1] if row < (len(inputs) - 1) and col < (len(inputs[0]) - 1) else None,
        "position_x": col,
        "position_y": row,
    }
    for row in range(0, len(inputs))
    for col in range(0, len(inputs[0]))
    if inputs[row][col]
]
