**FREE
// Native unit NDATAIMP (S2.91): data imports and exports, the program
// captured on PUB400 as GN91MAIN with DSPLY for its output.
// Source: golden capture tools/golden/native91 n91_data (PUB400, IBM i
// 7.5, 2026-11-18).
dcl-s total packed(7:0) import;
dcl-s label char(10) export inz('x');
dcl-pr addIt;
  n packed(7:0) const;
end-pr;
dcl-s line varchar(60);
line = %char(total) + ' ' + %trim(label);
addIt(5);
line += ' ' + %char(total) + ' ' + %trim(label);
total = 1;
label = 'y';
addIt(1);
line += ' ' + %char(total) + ' ' + %trim(label);
dsply line;
*inlr = *on;
