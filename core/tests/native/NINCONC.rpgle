**FREE
// Native unit NINCONC (S2.182): x IN an enumeration and %CONCAT, the
// program captured on PUB400 as GN182IC with DSPLY for its output.
// Source: golden capture tools/golden/native182 n182_inconcat (PUB400,
// IBM i 7.5, 2026-11-21).
dcl-enum color qualified;
  red 'R';
  green 'G';
  blue 'B';
end-enum;
dcl-enum size;
  small 1;
  big 9;
end-enum;
dcl-s c char(1) inz('G');
dcl-s n packed(3:0) inz(9);
dcl-s w char(4) inz('ab');
dcl-s line varchar(200);

if c in color;
  line = 'G-in ';
else;
  line = 'G-out ';
endif;
c = 'X';
if c in color;
  line += 'X-in ';
else;
  line += 'X-out ';
endif;
if n in size;
  line += '9-in|';
else;
  line += '9-out|';
endif;
line += %concat(', ' : 'x' : 'y' : 'z') + '|';
line += %concat(*none : 'a' : 'b') + '|';
line += %concat(*blank : 'a' : 'b') + '|';
line += %concat('-' : w : 'q' : w) + '|';
dsply (line);
*inlr = *on;
