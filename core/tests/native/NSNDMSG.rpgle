**FREE
// Native unit NSNDMSG (S2.175): SND-MSG *INFO, the program captured on
// PUB400 as GN175SM with DSPLY for its output.
// Source: golden capture tools/golden/native175 n175_sndmsg (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s line varchar(80) inz('start');

snd-msg *info 'just a note';
line += ' info';
dsply (line);
*inlr = *on;
