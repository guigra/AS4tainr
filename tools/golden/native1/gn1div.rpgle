**FREE
// Unmonitored division by zero: the escape message the caller sees.
dcl-s z packed(5:0) inz(0);
dcl-s q packed(5:0);
q = 10 / z;
*inlr = *on;
