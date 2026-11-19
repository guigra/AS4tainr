**FREE
// Native unit NINDCMP (S2.105): indicator fields compared, the program
// captured on PUB400 as GN105ICM with DSPLY for its output.
// Source: golden capture tools/golden/native105 n105_indcmp (PUB400, IBM
// i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s flag ind inz(*on);
dcl-s spare ind;
dcl-s k int(5);
dcl-s line varchar(80);
if flag = *on;
  line += 'a';
endif;
if flag = *off;
  line += 'X';
endif;
if *off = spare;
  line += 'b';
endif;
if flag = spare;
  line += 'X';
endif;
if flag <> spare;
  line += 'c';
endif;
spare = *on;
if flag = spare;
  line += 'd';
endif;
dow spare = *on;
  k += 1;
  if k = 3;
    spare = *off;
  endif;
enddo;
line += %char(k) + ' ' + local();
dsply (line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(10);
  end-pi;
  dcl-s mine ind inz(*off);
  if mine <> *on;
    return 'ok';
  endif;
  return 'no';
end-proc;
