**FREE
// Native unit NLOCFIL (S2.117): procedures with files of their own, the
// program captured on PUB400 as GN117LF with DSPLY for its output; its
// file is QGPL/GN117F here with the capture's six records (7 8 9 7 8 9:
// the capture's CREATE OR REPLACE kept an earlier run's three).
// Source: golden capture tools/golden/native117 n117_localfile (PUB400,
// IBM i 7.5, 2026-11-19).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);
line = %char(count()) + ' ' + %char(count()) + ' ' + %char(next())
       + %char(next()) + %char(next());
dsply (line);
*inlr = *on;

dcl-proc count;
  dcl-pi *n int(10);
  end-pi;
  dcl-f gn117f disk usage(*input) qualified extfile('QGPL/GN117F');
  dcl-ds rec likerec(gn117f.gn117f);
  dcl-s k int(10);
  read gn117f rec;
  dow not %eof(gn117f);
    k += 1;
    read gn117f rec;
  enddo;
  return k;
end-proc;

dcl-proc next;
  dcl-pi *n int(10);
  end-pi;
  dcl-f gn117f disk usage(*input) static qualified
                 extfile('QGPL/GN117F');
  dcl-ds rec likerec(gn117f.gn117f);
  read gn117f rec;
  return rec.n;
end-proc;
