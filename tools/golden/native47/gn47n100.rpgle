**FREE
// IBM i's nesting limit: an expression nested 100 levels (counting
// the outermost), compile only. tools/golden/native47.txt.
dcl-s k int(10);
k = ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((
(((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
)))))))))))))))))))))));
*inlr = *on;
