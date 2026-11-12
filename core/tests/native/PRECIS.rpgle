**FREE
// The default precision rules (IBM Docs "Example of Default Precision
// Rules"): FLD2 / (((FLD3 / 100) * FLD4) + FLD5) gets no decimal places,
// so the quotient is truncated to an integer; %DEC keeps decimals.
dcl-s fld1 packed(15:4);
dcl-s fld2 packed(15:2) inz(100);
dcl-s fld3 packed(5:2) inz(50);
dcl-s fld4 packed(9:4) inz(2);
dcl-s fld5 packed(9:4) inz(0.5);
fld1 = fld2 / (((fld3 / 100) * fld4) + fld5);
if fld1 = 66;
  dsply 'DEFAULT RULES OK';
else;
  dsply 'DEFAULT RULES FAIL';
endif;
fld1 = fld2 / (((%dec(fld3 / 100 : 15 : 4)) * fld4) + fld5);
if fld1 = 66.6666;
  dsply 'DEC RULES OK';
else;
  dsply 'DEC RULES FAIL';
endif;
*inlr = *on;
