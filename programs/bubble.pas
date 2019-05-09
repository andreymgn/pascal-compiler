program bubbleSort;
var
    a: array[0..4] of integer;
    i, j, x: integer;
begin
    a := [3, 25, 2, 69, 1];
    for i := 0 to 4 do
    begin
        for j := 4 downto i do
            if a[i-1] > a[j] then
            begin
                x := a[i-1];
                a[i-1] := a[j];
                a[j] := x;
            end;
    end;

    for i := 0 to 4 do
        writeln(a[i]);
end.
