**FREE
// Stage 2, S2.150: *OMIT arguments to CEE APIs (CEEDAYS, CEEDYWK,
// CEEDATE) with OPTIONS(*OMIT) feedback codes, and one call with a
// feedback field passed. Captured on PUB400 (tools/golden/native150.txt
// n150_omit) for the native unit NOMITCE.
ctl-opt dftactgrp(*no);
dcl-pr ceedays extproc('CEEDAYS') opdesc;
  indate char(255) const options(*varsize);
  picture char(255) const options(*varsize);
  lil int(10);
  fc char(12) options(*omit);
end-pr;
dcl-pr ceedywk extproc('CEEDYWK');
  lil int(10) const;
  wday int(10);
  fc char(12) options(*omit);
end-pr;
dcl-pr ceedate extproc('CEEDATE') opdesc;
  lil int(10) const;
  picture char(255) const options(*varsize);
  outdt char(80) options(*varsize);
  fc char(12) options(*omit);
end-pr;
dcl-s lil int(10);
dcl-s wday int(10);
dcl-s outdt char(80);
dcl-s fc char(12);
dcl-s line varchar(80);

ceedays('2026-01-31': 'YYYY-MM-DD': lil: *omit);
ceedywk(lil: wday: *omit);
ceedate(lil + 1: 'YYYY-MM-DD': outdt: fc);
line = %char(lil) + ' ' + %char(wday) + ' ' + %trimr(outdt);
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
