**FREE
// Native unit NINDCHR (S2.104): indicator fields as character data, the
// program captured on PUB400 as GN104ICH with DSPLY for its output.
// Source: golden capture tools/golden/native104 n104_indchar (PUB400, IBM
// i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s flag ind inz(*on);
dcl-s spare ind;
dcl-ds st qualified;
  done ind inz(*on);
end-ds;
dcl-s line varchar(80);
*in42 = *on;
line = %char(flag) + %char(spare) + ' ';
line += 'f=' + flag + ' s=' + spare + ' ' + flag + spare + *in42 + st.done;
line += ' ' + local();
dsply (line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(20);
  end-pi;
  dcl-s mine ind;
  mine = *on;
  return '<' + mine + %char(mine) + '>';
end-proc;
