PROGRAM BubbleSort;
VAR
    a: ARRAY[0..4] OF integer;
    i, j, x: integer;
BEGIN
    a := [3, 25, 2, 69, 1];
    FOR i := 0 TO 4 DO
    BEGIN
        FOR j := 4 DOWNTO i DO
            IF a[i-1] > a[j] THEN
            BEGIN
                x := a[i-1];
                a[i-1] := a[j];
                a[j] := x;
            END;
    END;

    FOR i := 0 TO 4 DO
        writeln(a[i]);
END.
