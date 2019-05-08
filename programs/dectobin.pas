program dectobin;
const 
  n = 8;
type
    iarray = array[integer] of integer;
var
  m,p:integer;
  B: array[1..n] of integer;
  C: array[1..n] of integer;
procedure DectoBin(x:integer; var A:iarray);
var 
  i:integer;
begin
  i:=1;
  while i<=8 do
    begin
      A[i]:=x mod 2;
      x:=x div 2;
      i:=i+1;
    end;
end;

function NimSum(x,y:integer):integer;
var
 binans :array[1..n] of integer;
 i,j,result:integer;
 
begin
  DectoBin(x,B);
  DectoBin(y,C);
  i:=1;
  j:=1;
  result:=0;
while i<=8 do
  begin
    if B[i]=C[i] then
      binans[i]:=0
    else
      binans[i]:=0;
    i:=i+1;
  end;
    i:=1;
  while i<=n do
    begin
    if (binans[i] = 1) and (i<>1) then
      begin
        for j := 1 to n do
          result:=result+2;
      end
    else if (binans[1] = 1) then
      result:=result+1;
    end;
        
  NimSum:=result;
end;
begin
  readln(m);
  readln(p);
  writeln(NimSum(m,p));
end.
