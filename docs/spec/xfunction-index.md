# X-Function index (Origin / 9.3)

Summary: Origin ships 582 X-Function definition files (extension .oxf) under its X-Functions folder. Each file declares one analysis or utility routine: its name, its input and output variables, option choices and defaults. This index lists all 582 with the category folder (the subfolder in which the file lives), a one-line purpose written in our own words, and a priority for the Rust rewrite. Priorities: P0 = needed for the owner's daily workflow (CH Instruments CV and chronoamperometry text, CSV, overlay plots, peak and charge integration, publication figures); P1 = common Origin use; P2 = later or probably never. Counts: P0 35, P1 94, P2 453.

Source (observed): every .oxf file under C:Program FilesOriginLabOriginX-Functions. The file extension is upper case on disk. Names are case-insensitive in Origin script. Purposes and priorities are our own judgement, not copied text. Several rows are interface helpers (dialog pages, graph tools, graph-drag events) rather than computations; they are marked P2 unless a P0/P1 tool depends on them. A full machine-readable dump (variables, defaults, choice lists) was produced in the git-ignored scratch folder re-workspace/scratch/analysis/xf.json.

How to read the table: Origin groups the folders as Data Exploration, Data Manipulation (with Gridding, Matrix, Worksheet subfolders), Database Access, Fitting, Graph Manipulation, Image (several subfolders), Import and Export, Mathematics, Miscellaneous, Plotting, Signal Processing (FFT, Wavelet), Spectroscopy (Peak Analyzer), Statistics (several subfolders), Utilities (File, System).

| Name | Category folder | Purpose | Priority |
|---|---|---|---|
| addtool_cluster | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_curve_deriv | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_curve_fft | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_curve_integ | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_curve_interp | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_curve_intersect | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_curve_stats | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_integ2 | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_quickfit | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_quickpeaks | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_rise_time | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| addtool_sigmoidal_fit | Data Exploration | Graph-tool version of an analysis (draw a rectangle on the plot to run it) | P1 |
| dlgRowColGoto | Data Exploration | Go to row/column | P2 |
| imageprofile | Data Exploration | Profile of an image/contour along a line | P2 |
| verticalcursor | Data Exploration | Vertical cursor reading values | P1 |
| vinc | Data Exploration | Average increment of a vector (used to derive sampling interval) | P1 |
| vinc_check | Data Exploration | Check evenness of X spacing | P1 |
| addsheet | Data Manipulation | Add sheet | P1 |
| assays | Data Manipulation | Assay template configuration | P2 |
| clrAllData | Data Manipulation | Clear all data in a sheet | P2 |
| copydata | Data Manipulation | Copy numeric data | P1 |
| curve translate | Data Manipulation | Interactively drag a curve vertically or horizontally | P1 |
| cxt | Data Manipulation | Shift a curve along X by an amount or to align a feature | P1 |
| findreplace | Data Manipulation | Find and replace values | P2 |
| levelcrossing | Data Manipulation | X positions where a curve crosses a given level | P1 |
| m2v | Data Manipulation | Matrix operation | P2 |
| map2c | Data Manipulation | Matrix operation | P2 |
| mc2ap | Data Manipulation | Matrix operation | P2 |
| mc2ri | Data Manipulation | Matrix operation | P2 |
| mcopy | Data Manipulation | Matrix operation | P2 |
| mks | Data Manipulation | Read data markers | P2 |
| mo2s | Data Manipulation | Matrix operation | P2 |
| mri2c | Data Manipulation | Matrix operation | P2 |
| ms2o | Data Manipulation | Matrix operation | P2 |
| newbook | Data Manipulation | Create workbook | P1 |
| newsheet | Data Manipulation | Create worksheet | P1 |
| rank | Data Manipulation | Flag points inside ranges | P2 |
| reduce_ex | Data Manipulation | Reduce to evenly spaced X | P1 |
| reducedup | Data Manipulation | Merge points with duplicate X | P1 |
| reducerows | Data Manipulation | Reduce every N rows with a statistic | P1 |
| reducexy | Data Manipulation | Reduce XY by groups of points, groups, or X increments | P1 |
| rtrim | Data Manipulation | Trim trailing blanks | P2 |
| subtract_line | Data Manipulation | Subtract a straight line defined by two points from a curve | P0 |
| subtract_ref | Data Manipulation | Subtract one curve from another, interpolating onto common X | P0 |
| trimright | Data Manipulation | Trim trailing missing values | P2 |
| v2m | Data Manipulation | Vector to matrix | P2 |
| vap2c | Data Manipulation | Complex-vector conversion helper | P2 |
| vc2ap | Data Manipulation | Complex-vector conversion helper | P2 |
| vc2ri | Data Manipulation | Complex-vector conversion helper | P2 |
| vfind | Data Manipulation | Find indices of a value in a vector | P2 |
| vri2c | Data Manipulation | Complex-vector conversion helper | P2 |
| vshift | Data Manipulation | Shift a vector with padding | P2 |
| xy_resample | Data Manipulation | System, scripting or housekeeping utility | P2 |
| xyz_resample | Data Manipulation | XYZ gridding or worksheet/matrix conversion | P2 |
| m2w | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| r2m | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| w2m | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| wexpand2m | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_regular | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_renka | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_renka_nag | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_shep | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_shep_nag | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_sparse | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyz_tps | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| XYZ2Mat | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| XYZ2MatLog | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| xyzgrid | Data Manipulation/Gridding | XYZ gridding or worksheet/matrix conversion | P2 |
| mCrop | Data Manipulation/Matrix | Crop a matrix | P2 |
| mdim | Data Manipulation/Matrix | Matrix operation | P2 |
| mexpand | Data Manipulation/Matrix | Matrix operation | P2 |
| mflip | Data Manipulation/Matrix | Matrix operation | P2 |
| mheaders | Data Manipulation/Matrix | Matrix operation | P2 |
| mproperty | Data Manipulation/Matrix | Matrix operation | P2 |
| mreplace | Data Manipulation/Matrix | Matrix operation | P2 |
| mrotate90 | Data Manipulation/Matrix | Matrix operation | P2 |
| msetvalue | Data Manipulation/Matrix | Matrix operation | P2 |
| mshrink | Data Manipulation/Matrix | Matrix operation | P2 |
| mtranspose | Data Manipulation/Matrix | Matrix operation | P2 |
| colcopy | Data Manipulation/Worksheet | Copy columns with metadata | P1 |
| colhide | Data Manipulation/Worksheet | Hide/unhide columns | P2 |
| colint | Data Manipulation/Worksheet | Set implicit X (initial value and increment) | P1 |
| colmask | Data Manipulation/Worksheet | Mask cells by condition or SD outliers | P1 |
| colmove | Data Manipulation/Worksheet | Move columns | P2 |
| colreverse | Data Manipulation/Worksheet | Reverse row order of columns | P1 |
| colshowx | Data Manipulation/Worksheet | Reveal the implicit X column | P2 |
| colslideshow | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| colsplit | Data Manipulation/Worksheet | Split columns by row grouping | P1 |
| colswap | Data Manipulation/Worksheet | Swap columns | P2 |
| filltext | Data Manipulation/Worksheet | Fill cells with random letters | P2 |
| getresults | Data Manipulation/Worksheet | Read result tree from a report | P2 |
| insertArrow | Data Manipulation/Worksheet | Graph layout/formatting helper | P2 |
| insertGraph | Data Manipulation/Worksheet | Graph layout/formatting helper | P2 |
| insertImg | Data Manipulation/Worksheet | Graph layout/formatting helper | P2 |
| insertNotes | Data Manipulation/Worksheet | Graph layout/formatting helper | P2 |
| insertSparklines | Data Manipulation/Worksheet | Graph layout/formatting helper | P2 |
| insertVar | Data Manipulation/Worksheet | Graph layout/formatting helper | P2 |
| merge_book | Data Manipulation/Worksheet | Matrix operation | P2 |
| patternD | Data Manipulation/Worksheet | Fill date/time pattern | P2 |
| patternN | Data Manipulation/Worksheet | Fill a column with a numeric pattern | P1 |
| patternT | Data Manipulation/Worksheet | Fill arbitrary text/number pattern | P2 |
| sparklines | Data Manipulation/Worksheet | System, scripting or housekeeping utility | P2 |
| updateEmbedGraphs | Data Manipulation/Worksheet | Refresh graphs embedded in cells | P2 |
| updateSparklines | Data Manipulation/Worksheet | System, scripting or housekeeping utility | P2 |
| w2vm | Data Manipulation/Worksheet | Virtual matrix from worksheet cells | P2 |
| w2xyz | Data Manipulation/Worksheet | Reshape grid-like data to XYZ columns | P2 |
| wAppend | Data Manipulation/Worksheet | Append worksheets into one | P1 |
| wautofill | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wautosize | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wcellcolor | Data Manipulation/Worksheet | Cell colours | P2 |
| wcellformat | Data Manipulation/Worksheet | Format cells | P2 |
| wcellgraph | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wcellmask | Data Manipulation/Worksheet | Mask cells in a range | P1 |
| wcellsel | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wclear | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wcolwidth | Data Manipulation/Worksheet | Column widths | P2 |
| wcopy | Data Manipulation/Worksheet | Duplicate a worksheet | P1 |
| wdeldup | Data Manipulation/Worksheet | Remove duplicate rows | P2 |
| wdelrows | Data Manipulation/Worksheet | Delete rows | P1 |
| wextract | Data Manipulation/Worksheet | Extract or mask rows by a condition (worksheet filter) | P0 |
| wjoinbycol | Data Manipulation/Worksheet | Join worksheets by matching a key column | P2 |
| wjoinbylabel | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wkeepdup | Data Manipulation/Worksheet | Keep duplicate rows | P2 |
| wks_to_wiki_table | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wks_update_link_table | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wmergexy | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wmove_sheet | Data Manipulation/Worksheet | Move sheet to another book | P2 |
| wmvsn | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wpivot | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wproperties | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wrcopy | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wreducecols | Data Manipulation/Worksheet | Reduce columns | P2 |
| wreducerows | Data Manipulation/Worksheet | Reduce rows by delete/merge | P2 |
| wreplace | Data Manipulation/Worksheet | Replace values | P2 |
| wrow2label | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wrowheight | Data Manipulation/Worksheet | Row heights | P2 |
| wrowhide | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wsort | Data Manipulation/Worksheet | Sort a worksheet by one or several columns | P0 |
| wsplit | Data Manipulation/Worksheet | Split worksheet columns into sheets | P2 |
| wsplit_book | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| wstackcol | Data Manipulation/Worksheet | Stack columns into one | P2 |
| wtranspose | Data Manipulation/Worksheet | Transpose worksheet | P1 |
| wunstackcol | Data Manipulation/Worksheet | Unstack grouped data | P2 |
| wxt | Data Manipulation/Worksheet | Worksheet or column manipulation helper | P2 |
| dbEdit | Database Access | Database import/query | P2 |
| dbImport | Database Access | Database import/query | P2 |
| dbInfo | Database Access | Database import/query | P2 |
| dbPreview | Database Access | Database import/query | P2 |
| findBase | Fitting | Detect the flat baseline region of a curve and fit a line to it | P1 |
| fitcmpdata | Fitting | Compare one model across two datasets (F-test) | P2 |
| fitcmpmodel | Fitting | Compare two models on one dataset (AIC, BIC, F-test) | P1 |
| Fitexp | Fitting | Exponential fit shortcut (1-3 decay terms, 1 growth) | P1 |
| fitfuncwiz | Fitting | Fitting function wizard | P2 |
| FitLinear | Fitting | Linear fit dialog (with optional X error) | P0 |
| fitLR | Fitting | Scriptable simple linear regression (intercept, slope, errors, r) | P0 |
| FitLRXErr | Fitting | Linear fit with errors in both X and Y | P2 |
| Fitmatrix | Fitting | Nonlinear surface fit on matrix | P2 |
| fitMR | Fitting | Scriptable multiple linear regression | P2 |
| FitNL | Fitting | Nonlinear curve fit dialog | P0 |
| FitODR | Fitting | Orthogonal distance regression fit for implicit models | P2 |
| Fitpeak | Fitting | Single-peak fit shortcut | P1 |
| fitpoly | Fitting | Scriptable polynomial regression | P0 |
| FitPolynomial | Fitting | Polynomial fit dialog | P0 |
| Fitsigmoidal | Fitting | Sigmoidal fit shortcut | P1 |
| Fitsurface | Fitting | Nonlinear surface fit on XYZ | P2 |
| funcRank | Fitting | Fit many models to one dataset and rank them | P2 |
| getnlr | Fitting | Read the fit tree from a fit report sheet | P2 |
| MR | Fitting | Multiple linear regression dialog | P2 |
| nlbegin | Fitting | Begin a scripted nonlinear fit session on XY data | P0 |
| nlbeginm | Fitting | Start a fit session on matrix data | P2 |
| nlbegino | Fitting | Start an implicit-function (ODR) fit session | P2 |
| nlbeginr | Fitting | Start a fit session on a data range | P1 |
| nlbeginz | Fitting | Start a fit session on XYZ data | P2 |
| nlend | Fitting | End the fit session and write the report | P0 |
| nlfit | Fitting | Run iterations of the active nonlinear fit session (LM or simplex) | P0 |
| nlfn | Fitting | Toggle automatic initial parameter guess | P1 |
| nlgui | Fitting | Read/set which fit outputs are produced | P1 |
| nlpara | Fitting | Open the fit parameters dialog | P2 |
| quickfit | Fitting | Quick fit helper | P2 |
| simcurve | Fitting | Generate a curve from a fit function and parameters | P1 |
| simsurface | Fitting | Generate a surface from a fit function | P2 |
| add inset layer | Graph Manipulation | Graph layout/formatting helper | P2 |
| add_graph_to_graph | Graph Manipulation | Graph layout/formatting helper | P2 |
| add_table_to_graph | Graph Manipulation | Graph layout/formatting helper | P2 |
| add_wks_to_graph | Graph Manipulation | Graph layout/formatting helper | P2 |
| add_xyscale_obj | Graph Manipulation | Graph layout/formatting helper | P2 |
| addline | Graph Manipulation | Draw a line on graph | P1 |
| axis_scrollbar | Graph Manipulation | Graph layout/formatting helper | P2 |
| axis_scroller | Graph Manipulation | Graph layout/formatting helper | P2 |
| CopySparkLine | Graph Manipulation | Graph layout/formatting helper | P2 |
| g2layout | Graph Manipulation | Graphs into layout | P2 |
| g2w | Graph Manipulation | Graph layout/formatting helper | P2 |
| gslider | Graph Manipulation | Slider tool | P2 |
| gxy2w | Graph Manipulation | Collect Y values at an X from all curves | P2 |
| insert_graph_to_layer | Graph Manipulation | Graph layout/formatting helper | P2 |
| insert_wks_to_layer | Graph Manipulation | Graph layout/formatting helper | P2 |
| layadd | Graph Manipulation | Add layer | P1 |
| layalign | Graph Manipulation | Graph layout/formatting helper | P2 |
| layarrange | Graph Manipulation | Arrange layers | P2 |
| laycolor | Graph Manipulation | Graph layout/formatting helper | P2 |
| laycopyscale | Graph Manipulation | Graph layout/formatting helper | P2 |
| layextract | Graph Manipulation | Graph layout/formatting helper | P2 |
| laylink | Graph Manipulation | Graph layout/formatting helper | P2 |
| laymanage | Graph Manipulation | Graph layout/formatting helper | P2 |
| laysetfont | Graph Manipulation | Set layer fonts | P1 |
| laysetpos | Graph Manipulation | Set layer position | P1 |
| laysetratio | Graph Manipulation | Set layer aspect ratio | P1 |
| laysetscale | Graph Manipulation | Set axis scales | P0 |
| laysetunit | Graph Manipulation | Set layer units | P1 |
| layswap | Graph Manipulation | Graph layout/formatting helper | P2 |
| laytoggle | Graph Manipulation | Graph layout/formatting helper | P2 |
| layzoom | Graph Manipulation | Graph layout/formatting helper | P2 |
| legendbox | Graph Manipulation | Graph layout/formatting helper | P2 |
| legendcat | Graph Manipulation | Graph layout/formatting helper | P2 |
| legendupdate | Graph Manipulation | Rebuild legend | P1 |
| lightcontroldlg | Graph Manipulation | Graph layout/formatting helper | P2 |
| merge_graph | Graph Manipulation | Merge graphs | P1 |
| newinset | Graph Manipulation | Graph layout/formatting helper | P2 |
| newpanel | Graph Manipulation | Graph layout/formatting helper | P2 |
| palApply | Graph Manipulation | Graph layout/formatting helper | P2 |
| pfit2l | Graph Manipulation | Graph layout/formatting helper | P2 |
| pickpts | Graph Manipulation | Pick data points from graph | P2 |
| speedmode | Graph Manipulation | Graph layout/formatting helper | P2 |
| imView | Image | Image utility | P2 |
| imgAutoLevel | Image/Adjustments | Image-processing operation | P2 |
| imgBalance | Image/Adjustments | Image-processing operation | P2 |
| imgBrightness | Image/Adjustments | Image-processing operation | P2 |
| imgColorlevel | Image/Adjustments | Image-processing operation | P2 |
| imgColorReplace | Image/Adjustments | Image-processing operation | P2 |
| imgContrast | Image/Adjustments | Image-processing operation | P2 |
| imgFuncLUT | Image/Adjustments | Image-processing operation | P2 |
| imgGamma | Image/Adjustments | Image-processing operation | P2 |
| imgHistcontrast | Image/Adjustments | Image-processing operation | P2 |
| imgHisteq | Image/Adjustments | Image-processing operation | P2 |
| imgHue | Image/Adjustments | Image-processing operation | P2 |
| imgInvert | Image/Adjustments | Image-processing operation | P2 |
| imgLevel | Image/Adjustments | Image-processing operation | P2 |
| imgSaturation | Image/Adjustments | Image-processing operation | P2 |
| image lines profile | Image/Analysis | Image conversion/analysis | P2 |
| imgHistogram | Image/Analysis | Image-processing operation | P2 |
| img2xyz | Image/Arithmetic Transform | Image conversion/analysis | P2 |
| imgBlend | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgMathfun | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgMorph | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgPixlog | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgReplaceBg | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgSimpleMath | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgSubtractBg | Image/Arithmetic Transform | Image-processing operation | P2 |
| imgSubtractInterpBG | Image/Arithmetic Transform | Image-processing operation | P2 |
| img2m | Image/Conversion | Image conversion/analysis | P2 |
| imgAutoBinary | Image/Conversion | Image-processing operation | P2 |
| imgBinary | Image/Conversion | Image-processing operation | P2 |
| imgC2gray | Image/Conversion | Image-processing operation | P2 |
| imgDynamicBinary | Image/Conversion | Image-processing operation | P2 |
| imgInfo | Image/Conversion | Image-processing operation | P2 |
| imgPalette | Image/Conversion | Image-processing operation | P2 |
| imgRGBmerge | Image/Conversion | Image-processing operation | P2 |
| imgRGBsplit | Image/Conversion | Image-processing operation | P2 |
| imgScale | Image/Conversion | Image-processing operation | P2 |
| imgThreshold | Image/Conversion | Image-processing operation | P2 |
| m2img | Image/Conversion | Image conversion/analysis | P2 |
| imgCrop | Image/Geometric Transform | Image-processing operation | P2 |
| imgFlip | Image/Geometric Transform | Image-processing operation | P2 |
| imgOffset | Image/Geometric Transform | Image-processing operation | P2 |
| imgResize | Image/Geometric Transform | Image-processing operation | P2 |
| imgRotate | Image/Geometric Transform | Image-processing operation | P2 |
| imgShear | Image/Geometric Transform | Image-processing operation | P2 |
| imgTrim | Image/Geometric Transform | Image-processing operation | P2 |
| imgAverage | Image/Spatial Filters | Image-processing operation | P2 |
| imgClear | Image/Spatial Filters | Image-processing operation | P2 |
| imgEdge | Image/Spatial Filters | Image-processing operation | P2 |
| imgGaussian | Image/Spatial Filters | Image-processing operation | P2 |
| imgMedian | Image/Spatial Filters | Image-processing operation | P2 |
| imgNoise | Image/Spatial Filters | Image-processing operation | P2 |
| imgSharpen | Image/Spatial Filters | Image-processing operation | P2 |
| imgUnsharpmask | Image/Spatial Filters | Image-processing operation | P2 |
| imgUserfilter | Image/Spatial Filters | Image-processing operation | P2 |
| batchProcess | Import and Export | Run an analysis template over many files | P1 |
| expASC | Import and Export | Export worksheet as ASCII | P1 |
| expGraph | Import and Export | Export graphs to image files | P0 |
| expImage | Import and Export | Export to a specific file format | P2 |
| expMatASC | Import and Export | Export to a specific file format | P2 |
| expNITDM | Import and Export | Export to a specific file format | P2 |
| expPDFw | Import and Export | Export to a specific file format | P2 |
| expWAV | Import and Export | Export to a specific file format | P2 |
| expWks | Import and Export | Export worksheet as image | P2 |
| img2GIF | Import and Export | Image conversion/analysis | P2 |
| impASC | Import and Export | Import delimited ASCII text with options | P0 |
| impBin2d | Import and Export | Import a specific instrument or third-party file format | P2 |
| impBRUKER | Import and Export | Import a specific instrument or third-party file format | P2 |
| impCDF | Import and Export | Import a specific instrument or third-party file format | P2 |
| impCSV | Import and Export | Import CSV | P0 |
| impDT | Import and Export | Import a specific instrument or third-party file format | P2 |
| impEDF | Import and Export | Import a specific instrument or third-party file format | P2 |
| impEP | Import and Export | Import a specific instrument or third-party file format | P2 |
| impExcel | Import and Export | Import Excel workbooks | P1 |
| impFamos | Import and Export | Import a specific instrument or third-party file format | P2 |
| impFile | Import and Export | Import using a saved filter | P1 |
| impFileSel | Import and Export | Import a specific instrument or third-party file format | P2 |
| impHDF5 | Import and Export | Import a specific instrument or third-party file format | P2 |
| impHEKA | Import and Export | Import a specific instrument or third-party file format | P2 |
| impIgorPro | Import and Export | Import a specific instrument or third-party file format | P2 |
| impImage | Import and Export | Import a specific instrument or third-party file format | P2 |
| impinfo | Import and Export | Read info about an import file | P2 |
| impISF | Import and Export | Import a specific instrument or third-party file format | P2 |
| impJCAMP | Import and Export | Import a specific instrument or third-party file format | P2 |
| impJNB | Import and Export | Import a specific instrument or third-party file format | P2 |
| impKG | Import and Export | Import a specific instrument or third-party file format | P2 |
| impMatlab | Import and Export | Import a specific instrument or third-party file format | P2 |
| impMDF | Import and Export | Import a specific instrument or third-party file format | P2 |
| impMNTB | Import and Export | Import a specific instrument or third-party file format | P2 |
| impMZXML | Import and Export | Import a specific instrument or third-party file format | P2 |
| impNetCDF | Import and Export | Import a specific instrument or third-party file format | P2 |
| impNIDIAdem | Import and Export | Import a specific instrument or third-party file format | P2 |
| impNITDM | Import and Export | Import a specific instrument or third-party file format | P2 |
| impODQ | Import and Export | Import a specific instrument or third-party file format | P2 |
| impOPJ | Import and Export | Import a specific instrument or third-party file format | P2 |
| imppClamp | Import and Export | Import a specific instrument or third-party file format | P2 |
| impPrism | Import and Export | Import a specific instrument or third-party file format | P2 |
| impSAS | Import and Export | Import a specific instrument or third-party file format | P2 |
| impSIE | Import and Export | Import a specific instrument or third-party file format | P2 |
| impSPC | Import and Export | Import a specific instrument or third-party file format | P2 |
| impSPE | Import and Export | Import a specific instrument or third-party file format | P2 |
| impSPSS | Import and Export | Import a specific instrument or third-party file format | P2 |
| impWav | Import and Export | Import a specific instrument or third-party file format | P2 |
| impWiz | Import and Export | Import wizard | P1 |
| insertImg2g | Import and Export | Graph layout/formatting helper | P2 |
| iwfilter | Import and Export | Create an import filter | P1 |
| pclampplotcontrol | Import and Export | pCLAMP plot control | P2 |
| plotpClamp | Import and Export | Plot-type creation helper | P2 |
| reimport2 | Import and Export | Re-import current file | P1 |
| word2ogw | Import and Export | Worksheet or column manipulation helper | P2 |
| avecurves | Mathematics | Average/concatenate several curves | P1 |
| averagexy | Mathematics | Average or concatenate multiple XY curves with common X options | P1 |
| bspline | Mathematics | Cubic B-spline (smoothing) interpolation at supplied X values | P2 |
| cnormalize | Mathematics | Normalize several curves one by one (adds reference-plot and picked-point options) | P1 |
| csetvalue | Mathematics | Fill a column from a formula (Set Column Values) | P0 |
| differentiate | Mathematics | Numerical derivative of a curve, orders 1-9, optional Savitzky-Golay smoothing | P0 |
| filter2 | Mathematics | Matrix operation | P2 |
| integ1 | Mathematics | Integrate a curve by the trapezoid rule; reports area, height, peak X, width, and optional cumulative curve | P0 |
| integ2 | Mathematics | Matrix operation | P2 |
| interp1 | Mathematics | Evaluate an interpolant of an XY curve at a supplied X list | P1 |
| interp1q | Mathematics | Fast linear interpolation at supplied X values | P1 |
| interp1trace | Mathematics | Interpolate a curve that doubles back (non-monotonic X) by following its trace | P1 |
| interp1xy | Mathematics | Resample an XY curve onto an evenly spaced X grid by interpolation/extrapolation | P0 |
| interp2 | Mathematics | Fitting or interpolation helper | P2 |
| interp3 | Mathematics | Fitting or interpolation helper | P2 |
| interpxyz | Mathematics | Fitting or interpolation helper | P2 |
| intersect | Mathematics | Intersections of two curves | P1 |
| marea | Mathematics | Matrix operation | P2 |
| mathtool | Mathematics | Add/subtract/multiply/divide/power a curve with a constant or another curve | P0 |
| medianflt2 | Mathematics | Matrix operation | P2 |
| minterp2 | Mathematics | Matrix operation | P2 |
| minverse | Mathematics | Matrix operation | P2 |
| normalize | Mathematics | Rescale a curve by a statistic, to [0,1] or [0,100], or to Z scores | P0 |
| polyarea | Mathematics | Area enclosed by a polygonal curve | P1 |
| reflection | Mathematics | Reflect data across an interval | P2 |
| rnormalize | Mathematics | Normalize worksheet columns in place | P1 |
| specialflt2 | Mathematics | Matrix operation | P2 |
| sph2cart | Mathematics | System, scripting or housekeeping utility | P2 |
| spline | Mathematics | Cubic spline interpolation at supplied X values | P1 |
| vcmath1 | Mathematics | Complex-vector conversion helper | P2 |
| vcmath2 | Mathematics | Complex-vector conversion helper | P2 |
| vmathtool | Mathematics | Same arithmetic on a single column | P1 |
| vnormalize | Mathematics | Normalize a plain vector | P2 |
| white_noise | Mathematics | Add Gaussian noise | P2 |
| xyzarea | Mathematics | System, scripting or housekeeping utility | P2 |
| dlgTheme | Miscellaneous | System, scripting or housekeeping utility | P2 |
| dm | Miscellaneous | System, scripting or housekeeping utility | P2 |
| doc2xf | Miscellaneous | System, scripting or housekeeping utility | P2 |
| getXYName | Miscellaneous | System, scripting or housekeeping utility | P2 |
| gui2xf | Miscellaneous | System, scripting or housekeeping utility | P2 |
| marker | Miscellaneous | Matrix operation | P2 |
| op_values | Miscellaneous | System, scripting or housekeeping utility | P2 |
| OpenBinWks | Miscellaneous | System, scripting or housekeeping utility | P2 |
| opFit | Miscellaneous | System, scripting or housekeeping utility | P2 |
| plot2bin | Miscellaneous | Plot-type creation helper | P2 |
| PlotCumulativeProb | Miscellaneous | System, scripting or housekeeping utility | P2 |
| plotellipse | Miscellaneous | Plot-type creation helper | P2 |
| xf2doc | Miscellaneous | System, scripting or housekeeping utility | P2 |
| xf2gui | Miscellaneous | System, scripting or housekeeping utility | P2 |
| xf2wiki | Miscellaneous | System, scripting or housekeeping utility | P2 |
| plot_gboxindexed | Plotting | Plot-type creation helper | P2 |
| plot_gindexed | Plotting | Plot-type creation helper | P2 |
| plot_kde2 | Plotting | Plot-type creation helper | P2 |
| plot_lineser | Plotting | Plot-type creation helper | P2 |
| plot_matrix | Plotting | Plot-type creation helper | P2 |
| plot_paretobin | Plotting | Plot-type creation helper | P2 |
| plot_paretoraw | Plotting | Plot-type creation helper | P2 |
| plot_prob | Plotting | Plot-type creation helper | P2 |
| plot_windrose | Plotting | Plot-type creation helper | P2 |
| plotbylabel | Plotting | Panels from column labels | P2 |
| plotgboxraw | Plotting | Plot-type creation helper | P2 |
| plotgroup | Plotting | Plot by groups | P1 |
| plotm | Plotting | Plot-type creation helper | P2 |
| plotmatrix | Plotting | Plot-type creation helper | P2 |
| plotms | Plotting | Plot-type creation helper | P2 |
| plotmyaxes | Plotting | Multi-axes plot | P2 |
| plotpiper | Plotting | Plot-type creation helper | P2 |
| plotstack | Plotting | Stacked graph | P1 |
| plotvm | Plotting | Plot-type creation helper | P2 |
| plotxy | Plotting | Create an XY plot with chosen style | P0 |
| plotxyz | Plotting | Plot-type creation helper | P2 |
| cohere | Signal Processing | System, scripting or housekeeping utility | P2 |
| conv | Signal Processing | Convolution | P2 |
| corr1 | Signal Processing | Cross/auto-correlation | P2 |
| corr2 | Signal Processing | System, scripting or housekeeping utility | P2 |
| decimate | Signal Processing | Lower the sampling rate | P2 |
| deconv | Signal Processing | Deconvolution | P2 |
| dfilter | Signal Processing | Design/apply IIR filter | P2 |
| dfilterapply | Signal Processing | Apply IIR coefficients | P2 |
| dfilterfcf | Signal Processing | Import/export IIR coefficient files | P2 |
| envelope | Signal Processing | Upper/lower envelope of a signal | P2 |
| fft_filter2 | Signal Processing | System, scripting or housekeeping utility | P2 |
| fft_filters | Signal Processing | FFT filtering: low/high/band pass, band block, threshold, parabolic low pass | P1 |
| hilbert | Signal Processing | Hilbert transform / analytic signal | P2 |
| msmooth | Signal Processing | Matrix operation | P2 |
| smooth | Signal Processing | Noise smoothing: adjacent average, Savitzky-Golay, percentile, FFT, Lowess, Loess, binomial | P0 |
| stft | Signal Processing | Short-time Fourier transform | P2 |
| fft1 | Signal Processing/FFT | Fast Fourier transform with window and amplitude/power conventions | P1 |
| fft2 | Signal Processing/FFT | System, scripting or housekeeping utility | P2 |
| ifft1 | Signal Processing/FFT | Inverse FFT | P1 |
| ifft2 | Signal Processing/FFT | System, scripting or housekeeping utility | P2 |
| unwrap | Signal Processing/FFT | System, scripting or housekeeping utility | P2 |
| cw_evaluate | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| cwt | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| dwt | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| dwt2 | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| idwt | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| idwt2 | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| mdwt | Signal Processing/Wavelet | Matrix operation | P2 |
| wtdenoise | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| wtsmooth | Signal Processing/Wavelet | Wavelet transform tool | P2 |
| blauto | Spectroscopy | Automatically place baseline anchor points | P1 |
| fitpeaks | Spectroscopy | Multiple peak fit starting from auto-found peaks (Gaussian/Lorentzian/Voigt) | P1 |
| nlfitpeaks | Spectroscopy | Multiple peak fit, general peak function | P1 |
| pa | Spectroscopy | Open the Peak Analyzer wizard (baseline, find, integrate or fit peaks) | P0 |
| paMultiY | Spectroscopy | Run a saved Peak Analyzer theme over many Y columns | P1 |
| pkFind | Spectroscopy | Locate peaks and their left/right feet on a curve | P0 |
| pa_alsbase | Spectroscopy/Peak Analyzer | Baseline by asymmetric least squares smoothing | P1 |
| pa_anchors | Spectroscopy/Peak Analyzer | Anchor-point picking events for Peak Analyzer | P2 |
| pa_basecreate | Spectroscopy/Peak Analyzer | Peak Analyzer baseline from anchor points (interpolation or fit) | P0 |
| pa_basemode | Spectroscopy/Peak Analyzer | Peak Analyzer page: choose baseline mode | P0 |
| pa_basesubtr | Spectroscopy/Peak Analyzer | Subtract the computed baseline from data | P0 |
| pa_basetreat | Spectroscopy/Peak Analyzer | Baseline treatment flags (subtract, fit with peaks, fix) | P1 |
| pa_epbase | Spectroscopy/Peak Analyzer | Baseline from weighted end-point fractions | P1 |
| pa_fit | Spectroscopy/Peak Analyzer | Peak Analyzer page: fit peaks | P1 |
| pa_goal | Spectroscopy/Peak Analyzer | Peak Analyzer page: choose the goal | P0 |
| pa_int | Spectroscopy/Peak Analyzer | Peak Analyzer page: integrate peaks and report area, center, height, FWHM | P0 |
| pa_peaks | Spectroscopy/Peak Analyzer | Peak Analyzer page: find/edit peaks | P0 |
| pa_xpsbase | Spectroscopy/Peak Analyzer | Shirley/Tougaard XPS background | P2 |
| ROCCurve | Statistics | Statistical test or multivariate method | P2 |
| ANOVAOneWay | Statistics/ANOVA | Statistical test or multivariate method | P2 |
| ANOVAOneWayRM | Statistics/ANOVA | Statistical test or multivariate method | P2 |
| ANOVAThreeWay | Statistics/ANOVA | Statistical test or multivariate method | P2 |
| ANOVATwoWay | Statistics/ANOVA | Statistical test or multivariate method | P2 |
| ANOVATwoWayRM | Statistics/ANOVA | Statistical test or multivariate method | P2 |
| colstats | Statistics/Descriptive Statistics | Descriptive statistics for each column | P0 |
| corrcoef | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| crosstab | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| DescStats | Statistics/Descriptive Statistics | Statistics-on-columns/rows dialog | P0 |
| DescStats_1 | Statistics/Descriptive Statistics | Row statistics dialog helper | P1 |
| discfreqs | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| distfit | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| diststats | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| freqcounts | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| gDescStats | Statistics/Descriptive Statistics | Descriptive statistics on graph data | P2 |
| grubbs | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| kstest | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| lillietest | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| mmoments | Statistics/Descriptive Statistics | Moments of a matrix | P2 |
| moments | Statistics/Descriptive Statistics | Moments, SE, skewness, kurtosis, CV | P1 |
| mquantiles | Statistics/Descriptive Statistics | Quantiles of a matrix | P2 |
| mstats | Statistics/Descriptive Statistics | Statistics of a matrix | P2 |
| NormalityTest | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| pcorrcoef | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| qtest | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| quantiles | Statistics/Descriptive Statistics | Min, quartiles, median, max, IQR of one vector | P1 |
| rowquantiles | Statistics/Descriptive Statistics | Quantiles by row | P2 |
| rowstats | Statistics/Descriptive Statistics | Descriptive statistics across rows | P1 |
| stats | Statistics/Descriptive Statistics | Descriptive statistics of one vector: mean, SD, N, min, max, sum, missing | P0 |
| swtest | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| twoDBinning | Statistics/Descriptive Statistics | Statistical test or multivariate method | P2 |
| OneSampleProportionTest | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| OneSampleTestVar | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| OneSampletTest | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| PairSampletTest | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| rowttest2 | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| rowttestpair | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| ttest1 | Statistics/Hypothesis Testing | One-sample t test | P1 |
| ttest2 | Statistics/Hypothesis Testing | Two-sample t test | P1 |
| ttestpair | Statistics/Hypothesis Testing | Paired t test | P1 |
| TwoSampleProportionTest | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| TwoSampleTestVar | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| TwoSampletTest | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| vartest1 | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| vartest2 | Statistics/Hypothesis Testing | Statistical test or multivariate method | P2 |
| discrim | Statistics/Multivariate Analysis | Statistical test or multivariate method | P2 |
| hcluster | Statistics/Multivariate Analysis | Statistical test or multivariate method | P2 |
| kmeans | Statistics/Multivariate Analysis | Statistical test or multivariate method | P2 |
| pca | Statistics/Multivariate Analysis | Statistical test or multivariate method | P2 |
| pls | Statistics/Multivariate Analysis | Statistical test or multivariate method | P2 |
| friedman | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| kstest2 | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| kwanova | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| mediantest | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| mwtest | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| NPHKSamples | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| NPHPairSample | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| NPHTwoSample | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| sign2 | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| signrank1 | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| signrank2 | Statistics/Nonparametric Tests | Statistical test or multivariate method | P2 |
| PSS_ANOVA1 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_proportionTest1 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_proportionTest2 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_tTest1 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_tTest2 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_tTestPair | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_varTest1 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| PSS_varTest2 | Statistics/Power and Sample Size | Statistical test or multivariate method | P2 |
| kaplanmeier | Statistics/Survival Analysis | Statistical test or multivariate method | P2 |
| phm_Cox | Statistics/Survival Analysis | Statistical test or multivariate method | P2 |
| weibullfit | Statistics/Survival Analysis | Statistical test or multivariate method | P2 |
| customMenu | Utilities | System, scripting or housekeeping utility | P2 |
| digitizer | Utilities | Digitize points from image | P2 |
| get_plot_sel | Utilities | System, scripting or housekeeping utility | P2 |
| get_wks_sel | Utilities | System, scripting or housekeeping utility | P2 |
| mkvid | Utilities | System, scripting or housekeeping utility | P2 |
| palEdit | Utilities | Graph layout/formatting helper | P2 |
| themeApply2g | Utilities | Apply a theme to graphs | P1 |
| themeApply2w | Utilities | Apply a theme to worksheets | P1 |
| themeEdit | Utilities | Edit theme | P2 |
| xop | Utilities | System, scripting or housekeeping utility | P2 |
| cmpfile | Utilities/File | System, scripting or housekeeping utility | P2 |
| dlgFile | Utilities/File | System, scripting or housekeeping utility | P2 |
| dlgPath | Utilities/File | System, scripting or housekeeping utility | P2 |
| dlgSave | Utilities/File | System, scripting or housekeeping utility | P2 |
| filelog | Utilities/File | System, scripting or housekeeping utility | P2 |
| findFiles | Utilities/File | System, scripting or housekeeping utility | P2 |
| findFolders | Utilities/File | System, scripting or housekeeping utility | P2 |
| imfcat | Utilities/File | Image utility | P2 |
| imgFile | Utilities/File | Image-processing operation | P2 |
| lvvicopy | Utilities/File | System, scripting or housekeeping utility | P2 |
| oftp | Utilities/File | System, scripting or housekeeping utility | P2 |
| template_saveas | Utilities/File | System, scripting or housekeeping utility | P2 |
| web2file | Utilities/File | Worksheet or column manipulation helper | P2 |
| cd | Utilities/System | Change working directory | P2 |
| cdset | Utilities/System | System, scripting or housekeeping utility | P2 |
| changeUFF | Utilities/System | System, scripting or housekeeping utility | P2 |
| debug_log | Utilities/System | System, scripting or housekeeping utility | P2 |
| dir | Utilities/System | List scripts | P2 |
| dlgChkList | Utilities/System | System, scripting or housekeeping utility | P2 |
| finddlg | Utilities/System | System, scripting or housekeeping utility | P2 |
| group_server | Utilities/System | System, scripting or housekeeping utility | P2 |
| groupmgr | Utilities/System | System, scripting or housekeeping utility | P2 |
| instOPX | Utilities/System | System, scripting or housekeeping utility | P2 |
| language | Utilities/System | System, scripting or housekeeping utility | P2 |
| lc | Utilities/System | List categories | P2 |
| lic | Utilities/System | System, scripting or housekeeping utility | P2 |
| lx | Utilities/System | List X-Functions | P2 |
| mkdir | Utilities/System | Matrix operation | P2 |
| op_change | Utilities/System | System, scripting or housekeeping utility | P2 |
| pb | Utilities/System | System, scripting or housekeeping utility | P2 |
| pe_cd | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_dir | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_load | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_mkdir | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_move | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_path | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_rename | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_rmdir | Utilities/System | Project-explorer or slide-show utility | P2 |
| pe_save | Utilities/System | Project-explorer or slide-show utility | P2 |
| pef_pptslide | Utilities/System | Project-explorer or slide-show utility | P2 |
| pef_slideshow | Utilities/System | Project-explorer or slide-show utility | P2 |
| pemp_pptslide | Utilities/System | Project-explorer or slide-show utility | P2 |
| pemp_slideshow | Utilities/System | Project-explorer or slide-show utility | P2 |
| pep_addshortcuts | Utilities/System | Project-explorer or slide-show utility | P2 |
| pesp_gotofolder | Utilities/System | Project-explorer or slide-show utility | P2 |
| saveProjectNoData | Utilities/System | System, scripting or housekeeping utility | P2 |
| updateUFF | Utilities/System | System, scripting or housekeeping utility | P2 |
| ux | Utilities/System | System, scripting or housekeeping utility | P2 |
