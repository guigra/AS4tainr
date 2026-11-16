**FREE
// Nested built-in functions: %ABS 99 times around a field (100 levels
// with the field's), compile only, lines split between tokens.
// tools/golden/native49.txt.
dcl-s k int(10) inz(-3);
k = %abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(
%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(
%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(
%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(
%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(
%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(
%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(%abs(k)))))))))))))))))))))))))))
))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))));
*inlr = *on;
