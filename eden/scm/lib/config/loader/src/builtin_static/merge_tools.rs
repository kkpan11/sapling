/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use staticconfig::StaticConfig;
use staticconfig::static_config;

/// Merge tools.
///
/// Should be loaded except when RCPATH environment variable is set.
pub static CONFIG: StaticConfig = static_config!("builtin:merge-tools" => r#"
[merge-tools]
kdiff3.args=--auto --L1 base --L2 local --L3 other $base $local $other -o $output
kdiff3.regkey=Software\KDiff3
kdiff3.regkeyalt=Software\Wow6432Node\KDiff3
kdiff3.regappend=\kdiff3.exe
kdiff3.fixeol=True
kdiff3.gui=True
kdiff3.diffargs=--L1 $plabel1 --L2 $clabel $parent $child

gvimdiff.args=--nofork -d -g -O $local $other $base
gvimdiff.regkey=Software\Vim\GVim
gvimdiff.regkeyalt=Software\Wow6432Node\Vim\GVim
gvimdiff.regname=path
gvimdiff.priority=-9
gvimdiff.diffargs=--nofork -d -g -O $parent $child

vimdiff.args=$local $other $base -c 'redraw | echomsg "hg merge conflict, type \":cq\" to abort vimdiff"'
vimdiff.check=changed
vimdiff.priority=-10

merge.check=conflicts
merge.priority=-100

gpyfm.gui=True

meld.gui=True
meld.args=--label='local' $local --label='merged' $base --label='other' $other -o $output
meld.check=changed
meld.diffargs=-a --label=$plabel1 $parent --label=$clabel $child

tkdiff.args=$local $other -a $base -o $output
tkdiff.gui=True
tkdiff.priority=-8
tkdiff.diffargs=-L $plabel1 $parent -L $clabel $child

xxdiff.args=--show-merged-pane --exit-with-merge-status --title1 local --title2 base --title3 other --merged-filename $output --merge $local $base $other
xxdiff.gui=True
xxdiff.priority=-8
xxdiff.diffargs=--title1 $plabel1 $parent --title2 $clabel $child

diffmerge.regkey=Software\SourceGear\SourceGear DiffMerge\
diffmerge.regkeyalt=Software\Wow6432Node\SourceGear\SourceGear DiffMerge\
diffmerge.regname=Location
diffmerge.priority=-7
diffmerge.args=-nosplash -merge -title1=local -title2=merged -title3=other $local $base $other -result=$output
diffmerge.check=changed
diffmerge.gui=True
diffmerge.diffargs=--nosplash --title1=$plabel1 --title2=$clabel $parent $child

p4merge.args=$base $local $other $output
p4merge.regkey=Software\Perforce\Environment
p4merge.regkeyalt=Software\Wow6432Node\Perforce\Environment
p4merge.regname=P4INSTROOT
p4merge.regappend=\p4merge.exe
p4merge.gui=True
p4merge.priority=-8
p4merge.diffargs=$parent $child

p4mergeosx.executable = /Applications/p4merge.app/Contents/MacOS/p4merge
p4mergeosx.args = $base $local $other $output
p4mergeosx.gui = True
p4mergeosx.priority=-8
p4mergeosx.diffargs=$parent $child

tortoisemerge.args=/base:$base /mine:$local /theirs:$other /merged:$output
tortoisemerge.regkey=Software\TortoiseSVN
tortoisemerge.regkeyalt=Software\Wow6432Node\TortoiseSVN
tortoisemerge.check=changed
tortoisemerge.gui=True
tortoisemerge.priority=-8
tortoisemerge.diffargs=/base:$parent /mine:$child /basename:$plabel1 /minename:$clabel

ecmerge.args=$base $local $other --mode=merge3 --title0=base --title1=local --title2=other --to=$output
ecmerge.regkey=Software\Elli\xc3\xa9 Computing\Merge
ecmerge.regkeyalt=Software\Wow6432Node\Elli\xc3\xa9 Computing\Merge
ecmerge.gui=True
ecmerge.diffargs=$parent $child --mode=diff2 --title1=$plabel1 --title2=$clabel

# editmerge is a small script shipped in contrib.
# It needs this config otherwise it behaves the same as internal:local
editmerge.args=$output
editmerge.check=changed
editmerge.premerge=keep

filemerge.executable=/Developer/Applications/Utilities/FileMerge.app/Contents/MacOS/FileMerge
filemerge.args=-left $other -right $local -ancestor $base -merge $output
filemerge.gui=True

filemergexcode.executable=/Applications/Xcode.app/Contents/Applications/FileMerge.app/Contents/MacOS/FileMerge
filemergexcode.args=-left $other -right $local -ancestor $base -merge $output
filemergexcode.gui=True

; Windows version of Beyond Compare
beyondcompare3.args=$local $other $base $output /ro /lefttitle=local /centertitle=base /righttitle=other /automerge /reviewconflicts /solo
beyondcompare3.regkey=Software\Scooter Software\Beyond Compare 3
beyondcompare3.regname=ExePath
beyondcompare3.gui=True
beyondcompare3.priority=-2
beyondcompare3.diffargs=/lro /lefttitle=$plabel1 /righttitle=$clabel /solo /expandall $parent $child

; Linux version of Beyond Compare
bcompare.args=$local $other $base -mergeoutput=$output -ro -lefttitle=parent1 -centertitle=base -righttitle=parent2 -outputtitle=merged -automerge -reviewconflicts -solo
bcompare.gui=True
bcompare.priority=-1
bcompare.diffargs=-lro -lefttitle=$plabel1 -righttitle=$clabel -solo -expandall $parent $child

; OS X version of Beyond Compare
bcomposx.executable = /Applications/Beyond Compare.app/Contents/MacOS/bcomp
bcomposx.args=$local $other $base -mergeoutput=$output -ro -lefttitle=parent1 -centertitle=base -righttitle=parent2 -outputtitle=merged -automerge -reviewconflicts -solo
bcomposx.gui=True
bcomposx.priority=-1
bcomposx.diffargs=-lro -lefttitle=$plabel1 -righttitle=$clabel -solo -expandall $parent $child

winmerge.args=/e /x /wl /ub /dl other /dr local $other $local $output
winmerge.regkey=Software\Thingamahoochie\WinMerge
winmerge.regkeyalt=Software\Wow6432Node\Thingamahoochie\WinMerge\
winmerge.regname=Executable
winmerge.check=changed
winmerge.gui=True
winmerge.priority=-10
winmerge.diffargs=/r /e /x /ub /wl /dl $plabel1 /dr $clabel $parent $child

araxis.regkey=SOFTWARE\Classes\TypeLib\{46799e0a-7bd1-4330-911c-9660bb964ea2}\7.0\HELPDIR
araxis.regappend=\ConsoleCompare.exe
araxis.priority=-2
araxis.args=/3 /a2 /wait /merge /title1:"Other" /title2:"Base" /title3:"Local :"$local $other $base $local $output
araxis.checkconflict=True
araxis.binary=True
araxis.gui=True
araxis.diffargs=/2 /wait /title1:$plabel1 /title2:$clabel $parent $child

diffuse.priority=-3
diffuse.args=$local $base $other
diffuse.gui=True
diffuse.diffargs=$parent $child

UltraCompare.regkey=Software\Microsoft\Windows\CurrentVersion\App Paths\UC.exe
UltraCompare.regkeyalt=Software\Wow6432Node\Microsoft\Windows\CurrentVersion\App Paths\UC.exe
UltraCompare.args = $base $local $other -title1 base -title3 other
UltraCompare.priority = -2
UltraCompare.gui = True
UltraCompare.binary = True
UltraCompare.check = conflicts,changed
UltraCompare.diffargs=$child $parent -title1 $clabel -title2 $plabel1
"#);
