**FREE
// Native unit NINDPATH (S2.144): indicator subfields by path, the
// program captured on PUB400 as GN144IP with DSPLY for its output.
// Source: golden capture tools/golden/native144 n144_indpath (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-ds val_t qualified template;
  isNull ind inz(*on);
  text varchar(10) inz;
end-ds;
dcl-ds list qualified;
  ev likeds(val_t) dim(3) inz(*likeds);
end-ds;
dcl-s i int(10) inz(2);
dcl-s line varchar(80);

line = list.ev(1).isNull + list.ev(i).isNull;
list.ev(2).isNull = *off;
line += list.ev(i).isNull;
if list.ev(3).isNull;
  line += 'y';
endif;
if not list.ev(i).isNull;
  line += 'n';
endif;
dsply (line);
*inlr = *on;
