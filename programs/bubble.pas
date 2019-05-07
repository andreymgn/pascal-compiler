program bubbleSort;
var
    toSort: array[0..5] of integer;
    passChanges: boolean;
    counter, index, temp: integer;
begin
    toSort := [3, 25, 2, 69, 1];
    passChanges := true;
    while passChanges do
    begin
        index := 0;
        passChanges := false;
        for index := 0 to 4 do
        begin
            if (toSort[index] > toSort[index +1]) then
            begin
                temp := toSort[index + 1];
                toSort[index + 1] := toSort[index];
                toSort[index] := temp;
                passChanges := true;
            end;
        end;
    end;

    for counter := 1 to 5 do
        writeln(toSort[counter]);
end.
